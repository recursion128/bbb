use super::*;

#[test]
fn carving_mask_uses_vanilla_xyz_index_and_trims_zero_words() {
    let mut mask = CarvingMask::new(-64, 384);
    assert!(mask.set_if_absent(0, -64, 0));
    assert!(!mask.set_if_absent(0, -64, 0));
    assert!(mask.set_if_absent(15, -64, 15));
    assert!(mask.set_if_absent(0, -63, 0));
    assert_eq!(mask.into_trimmed_longs(), vec![1, 0, 0, i64::MIN, 1]);
}
