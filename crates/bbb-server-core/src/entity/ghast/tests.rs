use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GHAST_ID;

use crate::entity::GeneralLivingData;

#[test]
fn ghast_retained_state_defaults() {
    let state = GeneralLivingData::new(VANILLA_ENTITY_TYPE_GHAST_ID)
        .unwrap()
        .ghast_retained()
        .unwrap();
    assert_eq!(state.explosion_power(), 1);
    assert!(!state.charging());
}
