use super::*;

#[test]
fn glowstone_charges_an_anchor_until_four() {
    let empty = default_state("minecraft:respawn_anchor");
    assert!(respawn_anchor::is_respawn_anchor(empty));
    assert_eq!(respawn_anchor::charges(empty), Some(0));
    assert!(respawn_anchor::can_be_charged(empty));
    assert!(respawn_anchor::is_respawn_fuel(Some(
        bbb_protocol::item_types::VANILLA_ITEM_GLOWSTONE_ID
    )));
    assert!(!respawn_anchor::is_respawn_fuel(None));

    let full = respawn_anchor::with_charges(empty, respawn_anchor::MAX_CHARGES)
        .expect("charges=4 is a legal state");
    assert_eq!(respawn_anchor::charges(full), Some(4));
    assert!(!respawn_anchor::can_be_charged(full));
}
