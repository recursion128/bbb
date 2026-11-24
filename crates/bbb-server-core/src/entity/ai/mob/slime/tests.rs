use crate::random::GameRandom;

use super::SlimeRuntime;
use crate::entity::ai::mob::{MobAiContext, MobAiStepKindResult};
use crate::entity::{EntityIdCounter, EntityStore};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID;

#[test]
fn slime_autonomous_runtime_move_control_hops_and_uses_magma_delay() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, Default::default())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    entity.set_on_ground(true);
    assert!(crate::entity::slime::set_size(entity, 2, true));
    let mut context = MobAiContext::from_living(
        1,
        0,
        Default::default(),
        0.0,
        0.0,
        entity.living().unwrap(),
        GameRandom::with_seed(1),
    );
    context.body.movement_speed = 0.5;
    let mut runtime = SlimeRuntime::new();
    runtime.set_direction(90.0, false);
    runtime.set_wanted_movement(1.0);
    let MobAiStepKindResult::Slime {
        jump_sound: Some(sound),
        ..
    } = runtime.move_control_tick(&mut context, entity)
    else {
        panic!("expected slime jump sound");
    };
    assert_eq!(context.controls.speed, 0.5);
    assert!(context.controls.jump_requested);
    assert_eq!(sound.resource_id, "minecraft:entity.magma_cube.jump");
    context.apply(entity.living_mut().unwrap());
    assert_eq!(entity.living().unwrap().speed(), 0.5);
}
