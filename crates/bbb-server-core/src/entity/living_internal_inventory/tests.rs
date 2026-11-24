use super::*;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
    VANILLA_ENTITY_TYPE_LLAMA_ID,
};

#[test]
fn cargo_capacity_tracks_chest_and_llama_strength_without_equipment_slots() {
    assert_eq!(
        LivingInternalInventory::new(VANILLA_ENTITY_TYPE_ALLAY_ID)
            .unwrap()
            .slots()
            .len(),
        1
    );
    assert_eq!(
        LivingInternalInventory::new(VANILLA_ENTITY_TYPE_HORSE_ID)
            .unwrap()
            .slots()
            .len(),
        0
    );
    let mut donkey = LivingInternalInventory::new(VANILLA_ENTITY_TYPE_DONKEY_ID).unwrap();
    donkey.resize_for_horse(HorsePhenotypeState::chested(true));
    assert_eq!(donkey.slots().len(), 15);
    let mut llama = LivingInternalInventory::new(VANILLA_ENTITY_TYPE_LLAMA_ID).unwrap();
    llama.resize_for_horse(HorsePhenotypeState::llama(true, 3, 0));
    assert_eq!(llama.slots().len(), 9);
}
