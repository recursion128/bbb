use super::*;

#[test]
fn set_seed_defaults_clear_and_reset_match_the_official_mutators() {
    let mut sequences = RandomSequences::new();
    sequences.get("minecraft:blocks/stone", 0);
    sequences.get("minecraft:blocks/oak_leaves", 0);
    assert_eq!(sequences.entries().count(), 2);

    // `RandomSequences.setSeedDefaults` (`RandomSequences.java:64-68`).
    sequences.set_seed_defaults(7, false, false);
    assert_eq!(sequences.salt(), 7);
    assert!(!sequences.include_world_seed());
    assert!(!sequences.include_sequence_id());
    assert_eq!(sequences.entries().count(), 2);

    // `RandomSequences.clear` (`:70-74`) reports what it removed.
    assert_eq!(sequences.clear(), 2);
    assert_eq!(sequences.entries().count(), 0);
    assert_eq!(sequences.salt(), 7);

    // Two-arg `reset` (`:76-78`) uses the current defaults, live or not.
    sequences.reset("minecraft:blocks/stone", 1);
    assert_eq!(sequences.entries().count(), 1);

    // Five-arg `reset` (`:80-82`) seeds from an explicit trio without
    // moving the defaults.
    sequences.reset_with("minecraft:blocks/stone", 0, 0, true, true);
    assert_eq!(sequences.entries().count(), 1);
    assert_eq!(sequences.salt(), 7);
    assert!(!sequences.include_world_seed());
    assert!(!sequences.include_sequence_id());
}
