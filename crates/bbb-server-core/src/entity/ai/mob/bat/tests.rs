use crate::random::GameRandom;
use bbb_protocol::packets::Vec3d;

use crate::entity::living::LivingEntityState;

use super::select_target;
use crate::entity::ai::mob::MobAiContext;

fn context(seed: i64) -> MobAiContext {
    MobAiContext::from_living(
        1,
        1,
        Vec3d::default(),
        0.0,
        0.0,
        &LivingEntityState::new("minecraft:bat").unwrap(),
        GameRandom::with_seed(seed),
    )
}

#[test]
fn bat_autonomous_runtime_target_stays_within_the_wander_box() {
    let position = Vec3d {
        x: 10.5,
        y: 64.0,
        z: -7.5,
    };
    for seed in 0..64 {
        let target = select_target(position, &mut context(seed));
        assert!((4..=16).contains(&target.x), "{target:?}");
        assert!((62..=67).contains(&target.y), "{target:?}");
        assert!((-14..=-2).contains(&target.z), "{target:?}");
    }
}
