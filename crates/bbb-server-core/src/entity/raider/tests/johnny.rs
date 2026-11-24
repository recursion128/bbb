use super::*;

#[test]
fn johnny_cold_read_defaults_and_true_only_writer_are_exact() {
    for (saved, expected) in [(None, false), (Some(false), false), (Some(true), true)] {
        let source: Vec<_> = saved
            .map(|value| entry("Johnny", NbtSummaryValue::Byte(i8::from(value))))
            .into_iter()
            .collect();
        let state = read_save_data(VANILLA_ENTITY_TYPE_VINDICATOR_ID, &source);
        assert_eq!(state.johnny(), Some(expected));
        let mut written = Vec::new();
        write_save_data(&mut written, &state);
        assert_eq!(
            field(&written, "Johnny"),
            expected.then_some(&NbtSummaryValue::Byte(1))
        );
        assert_eq!(
            written
                .iter()
                .filter(|entry| entry.name == "Johnny")
                .count(),
            usize::from(expected)
        );
    }
    let mut marked = RaiderRetainedState::new(VANILLA_ENTITY_TYPE_VINDICATOR_ID).unwrap();
    marked.mark_johnny();
    let mut saved = Vec::new();
    write_save_data(&mut saved, &marked);
    assert_eq!(field(&saved, "Johnny"), Some(&NbtSummaryValue::Byte(1)));
    assert_eq!(
        read_save_data(VANILLA_ENTITY_TYPE_VINDICATOR_ID, &[]).johnny(),
        Some(false)
    );
}
