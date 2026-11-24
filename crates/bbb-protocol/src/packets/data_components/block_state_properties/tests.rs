use super::BlockStateProperties;
use crate::JavaString;

fn values(map: &BlockStateProperties) -> Vec<String> {
    map.iter().map(|(_, value)| value.to_jdk_string()).collect()
}

#[test]
fn source_order_keeps_first_insertion_with_last_value() {
    let raw = vec![
        (JavaString::from("b"), JavaString::from("b-old")),
        (JavaString::from("a"), JavaString::from("a")),
        (JavaString::from("?"), JavaString::from("?")),
        (JavaString::from("b"), JavaString::from("b-new")),
    ];
    assert_eq!(
        values(&BlockStateProperties::from_entries(raw)),
        ["b-new", "a", "?"]
    );
}

#[test]
fn map_equality_ignores_iteration_order() {
    let json = BlockStateProperties::from_entries(vec![
        ("a".into(), "first".into()),
        ("?".into(), "question".into()),
        ("a".into(), "last".into()),
    ]);
    let nbt = BlockStateProperties::from_entries(vec![
        ("?".into(), "question".into()),
        ("a".into(), "last".into()),
    ]);
    assert_eq!(json, nbt);
    assert_eq!(json.get_str("a").unwrap().to_jdk_string(), "last");
    let raw = BlockStateProperties::from_entries([
        (vec![0xd800].into(), vec![0xd801, 0x41].into()),
        ("literal".into(), "\\uD800".into()),
    ]);
    let encoded = serde_json::to_string(&raw).unwrap();
    assert_eq!(
        serde_json::from_str::<BlockStateProperties>(&encoded).unwrap(),
        raw
    );
    let display = crate::to_display_json_value(&raw).unwrap();
    assert_eq!(
        display,
        serde_json::json!({"�": "�A", "literal": "\\uD800"})
    );
}
