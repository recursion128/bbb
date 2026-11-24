use super::*;
use crate::entity::{EntityIdCounter, EntityStore};
use bbb_protocol::packets::Vec3d;

#[test]
fn fox_constructor_installs_only_defend_trusted_and_first_configuration_is_idempotent() {
    for snow in [false, true] {
        let mut store = EntityStore::new(73, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                VANILLA_ENTITY_TYPE_FOX_ID,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
            )
            .unwrap();
        assert_eq!(
            store
                .mob_ai(id)
                .unwrap()
                .target_selector
                .debug_info()
                .goals
                .len(),
            1
        );
        assert_eq!(
            store.mob_ai(id).unwrap().target_selector.debug_info().goals[0].name,
            "NearestAttackableTargetGoal"
        );
        assert!(crate::entity::fox::set_spawn_variant(
            store.get_mut(id).unwrap(),
            snow
        ));
        assert!(crate::entity::configure_fox_target_goals(
            store.get_mut(id).unwrap()
        ));
        store.refresh_retained_mob_targets(id);
        let priorities: Vec<_> = store
            .mob_ai(id)
            .unwrap()
            .target_selector
            .debug_info()
            .goals
            .into_iter()
            .map(|goal| goal.priority)
            .collect();
        assert_eq!(
            priorities,
            if snow {
                vec![3, 4, 6, 6]
            } else {
                vec![3, 4, 4, 6]
            }
        );
        assert!(crate::entity::fox::set_spawn_variant(
            store.get_mut(id).unwrap(),
            !snow
        ));
        assert!(crate::entity::configure_fox_target_goals(
            store.get_mut(id).unwrap()
        ));
        store.refresh_retained_mob_targets(id);
        assert_eq!(
            store
                .mob_ai(id)
                .unwrap()
                .target_selector
                .debug_info()
                .goals
                .len(),
            4
        );
    }
}
