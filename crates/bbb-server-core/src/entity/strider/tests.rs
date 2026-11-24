use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_STRIDER_ID};

use crate::entity::GeneralLivingData;

#[test]
fn strider_environment_runtime_state_census_and_defaults_are_exact() {
    let owned = VANILLA_ENTITY_TYPES
        .iter()
        .filter_map(|entity_type| {
            GeneralLivingData::new(entity_type.id)
                .and_then(|data| data.strider_state())
                .map(|_| entity_type.id)
        })
        .collect::<Vec<_>>();
    assert_eq!(owned, vec![VANILLA_ENTITY_TYPE_STRIDER_ID]);

    let state = GeneralLivingData::new(VANILLA_ENTITY_TYPE_STRIDER_ID)
        .unwrap()
        .strider_state()
        .unwrap();
    assert_eq!(state.boost_time(), 0);
    assert!(!state.suffocating());
}
