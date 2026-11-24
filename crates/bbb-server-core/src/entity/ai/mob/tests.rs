use crate::entity::GeneralLivingData;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_MANNEQUIN_ID};

#[test]
fn mob_ai_runtime_covers_all_general_mobs_but_not_mannequin() {
    let general: Vec<_> = VANILLA_ENTITY_TYPES
        .iter()
        .filter_map(|facts| GeneralLivingData::new(facts.id))
        .collect();
    assert_eq!(general.len(), 90);
    assert_eq!(general.iter().filter(|data| data.is_mob()).count(), 89);
    assert!(!GeneralLivingData::new(VANILLA_ENTITY_TYPE_MANNEQUIN_ID)
        .unwrap()
        .is_mob());
}
