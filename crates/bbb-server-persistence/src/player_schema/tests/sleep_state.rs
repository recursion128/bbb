use super::*;

fn field(raw: &NamedCompound, name: &str) -> Option<NbtSummaryValue> {
    raw.entries()
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value.clone())
}

#[test]
fn first_save_authors_live_sleep_timer_and_optional_bed_position() {
    let uuid = Uuid::from_u128(0xbed);
    let mut patch = created_patch(uuid);
    patch.created.sleep_timer = 100;
    patch.created.sleeping_pos = Some([-12, 64, 31]);
    let document = created_player_document(&patch).unwrap();
    assert_eq!(
        field(&document, "SleepTimer"),
        Some(NbtSummaryValue::Short(100))
    );
    assert_eq!(
        field(&document, "sleeping_pos"),
        Some(NbtSummaryValue::IntArray(vec![-12, 64, 31]))
    );
    let parsed = Java26PlayerData::parse(document, uuid).unwrap();
    assert_eq!(parsed.sleep_timer(), 100);
    assert_eq!(parsed.sleeping_pos(), Some([-12, 64, 31]));
    patch.created.sleep_timer = 0;
    patch.created.sleeping_pos = None;
    let awake = created_player_document(&patch).unwrap();
    assert_eq!(field(&awake, "SleepTimer"), Some(NbtSummaryValue::Short(0)));
    assert_eq!(field(&awake, "sleeping_pos"), None);
    assert_eq!(
        Java26PlayerData::parse(awake, uuid).unwrap().sleeping_pos(),
        None
    );
}

#[test]
fn retained_patch_replaces_old_sleep_state_and_waking_removes_the_stale_position() {
    let uuid = Uuid::from_u128(0xbed2);
    let mut raw = canonical(uuid);
    replace(&mut raw, "SleepTimer", NbtSummaryValue::Short(90));
    raw.entries_mut()
        .push(e("sleeping_pos", NbtSummaryValue::IntArray(vec![1, 2, 3])));
    raw.entries_mut()
        .push(e("unowned", NbtSummaryValue::String("retained".into())));
    let retained = Java26PlayerData::parse(raw, uuid).unwrap();
    let mut patch = created_patch(uuid);
    patch.created.sleep_timer = 109;
    patch.created.sleeping_pos = Some([4, 5, 6]);
    let rewritten = retained
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    let loaded = Java26PlayerData::parse(rewritten, uuid).unwrap();
    assert_eq!(loaded.sleep_timer(), 109);
    assert_eq!(loaded.sleeping_pos(), Some([4, 5, 6]));
    patch.created.sleep_timer = 0;
    patch.created.sleeping_pos = None;
    patch.warden_spawn_tracker = Some(Java26WardenSpawnTracker::FRESH);
    let awake = retained
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert_eq!(field(&awake, "SleepTimer"), Some(NbtSummaryValue::Short(0)));
    assert_eq!(field(&awake, "sleeping_pos"), None);
    assert_eq!(
        field(&awake, "unowned"),
        Some(NbtSummaryValue::String("retained".into()))
    );
    let loaded = Java26PlayerData::parse(awake, uuid).unwrap();
    assert_eq!(loaded.sleep_timer(), 0);
    assert_eq!(loaded.sleeping_pos(), None);
    assert_eq!(retained.sleep_timer(), 90);
    assert_eq!(retained.sleeping_pos(), Some([1, 2, 3]));
}

#[test]
fn sleep_state_keeps_any_short_timer_and_drops_a_malformed_bed_position() {
    let uuid = Uuid::from_u128(0xbed3);
    for timer in [i16::MIN, -1, 0, 109, 110, i16::MAX] {
        let mut raw = canonical(uuid);
        replace(&mut raw, "SleepTimer", NbtSummaryValue::Short(timer));
        assert_eq!(
            Java26PlayerData::parse(raw, uuid).unwrap().sleep_timer(),
            timer
        );
    }
    for (value, timer) in [
        (NbtSummaryValue::Int(10), 10),
        (NbtSummaryValue::String("10".into()), 0),
    ] {
        let mut raw = canonical(uuid);
        replace(&mut raw, "SleepTimer", value);
        assert_eq!(
            Java26PlayerData::parse(raw, uuid).unwrap().sleep_timer(),
            timer
        );
    }
    for value in [
        NbtSummaryValue::IntArray(vec![1, 2]),
        NbtSummaryValue::IntArray(vec![1, 2, 3, 4]),
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Int(1),
            NbtSummaryValue::Int(2),
            NbtSummaryValue::Int(3),
        ]),
    ] {
        let mut raw = canonical(uuid);
        raw.entries_mut().push(e("sleeping_pos", value));
        assert_eq!(
            Java26PlayerData::parse(raw, uuid).unwrap().sleeping_pos(),
            None
        );
    }
    let mut patch = created_patch(uuid);
    for timer in [i16::MIN, -1, 110, i16::MAX] {
        patch.created.sleep_timer = timer;
        let document = created_player_document(&patch).unwrap();
        assert_eq!(
            Java26PlayerData::parse(document, uuid)
                .unwrap()
                .sleep_timer(),
            timer
        );
        let retained = Java26PlayerData::parse(canonical(uuid), uuid).unwrap();
        let document = retained
            .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
            .unwrap();
        assert_eq!(
            Java26PlayerData::parse(document, uuid)
                .unwrap()
                .sleep_timer(),
            timer
        );
    }
}
