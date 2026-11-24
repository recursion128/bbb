use super::*;

#[test]
fn resource_json_preserves_surrogates_and_merges_only_adjacent_pairs() {
    for (source, expected) in [
        (r#""\uD800""#, vec![0xd800]),
        (r#""\uDFFF""#, vec![0xdfff]),
        (r#""\uD800\uDC00""#, vec![0x10000]),
        (r#""\uD800\uD801\uDC00""#, vec![0xd800, 0x10400]),
        (r#""\uDC00\uD800""#, vec![0xdc00, 0xd800]),
        (r#""\\uD800""#, "\\uD800".chars().map(u32::from).collect()),
        (r#""Aé中𐀀\n""#, vec![65, 233, 0x4e2d, 0x10000, 10]),
    ] {
        let value = ResourceJsonValue::from_slice(source.as_bytes()).unwrap();
        let string = value.as_string().unwrap();
        assert_eq!(
            string.codepoints().collect::<Vec<_>>(),
            expected,
            "{source}"
        );
        assert_eq!(
            string.as_str().is_some(),
            !expected
                .iter()
                .any(|point| (0xd800..=0xdfff).contains(point)),
            "{source}"
        );
    }
}

#[test]
fn resource_json_duplicate_decoded_keys_keep_first_position_and_last_value() {
    let value = ResourceJsonValue::from_slice(
        r#"{"a":1,"\uD800":2,"𐀀":3,"\u0061":4,"\ud800":5,"\uD800\uDC00":6}"#.as_bytes(),
    )
    .unwrap();
    let entries = value.as_object().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0.as_str(), Some("a"));
    assert_eq!(entries[0].1.as_i64(), Some(4));
    assert_eq!(entries[1].0.codepoints().collect::<Vec<_>>(), [0xd800]);
    assert_eq!(entries[1].1.as_i64(), Some(5));
    assert_eq!(entries[2].0.as_str(), Some("𐀀"));
    assert_eq!(entries[2].1.as_i64(), Some(6));
    assert_eq!(value.get("a").unwrap().as_i64(), Some(4));
    assert!(value.get("missing").is_none());
}

#[test]
fn resource_json_queries_keep_json_types_distinct() {
    let value = ResourceJsonValue::from_slice(
        br#"{"array":[65,66],"bool":true,"null":null,"int":-3,"float":1.25,"str":"abc","uint":18446744073709551615}"#,
    )
    .unwrap();
    assert_eq!(value.get("array").unwrap().as_array().unwrap().len(), 2);
    assert!(!value.get("array").unwrap().is_string());
    assert!(serde_json::from_str::<RawString>("[65,66]").is_err());
    assert_eq!(value.get("bool").unwrap().as_bool(), Some(true));
    assert_eq!(value.get("int").unwrap().as_i64(), Some(-3));
    assert_eq!(value.get("float").unwrap().as_f64(), Some(1.25));
    assert_eq!(value.get("str").unwrap().as_str(), Some("abc"));
    assert_eq!(value.get("uint").unwrap().as_i64(), None);
    assert_eq!(value.get("null"), Some(&ResourceJsonValue::Null));
    assert_eq!(value.get("str").unwrap().as_bool(), None);
    assert_eq!(value.get("bool").unwrap().as_i64(), None);
    assert_eq!(value.get("int").unwrap().as_str(), None);
    assert_eq!(value.get("int").unwrap().as_object(), None);
}

#[test]
fn resource_json_numbers_keep_source_spelling_and_defer_numeric_projection() {
    let source = br#"{"first":1e2,"exponent":1E+02,"negative_zero":-0,"small":0.0000010,"precise":9007199254740993,"large":18446744073709551616,"huge":1e400,"text":"1e2","first":-0.0}"#;
    let value = ResourceJsonValue::from_slice(source).unwrap();
    let entries = value.as_object().unwrap();
    assert_eq!(entries.len(), 8);
    assert_eq!(entries[0].0.as_str(), Some("first"));
    for (key, expected) in [
        ("first", "-0.0"),
        ("exponent", "1E+02"),
        ("negative_zero", "-0"),
        ("small", "0.0000010"),
        ("precise", "9007199254740993"),
        ("large", "18446744073709551616"),
        ("huge", "1e400"),
    ] {
        assert_eq!(
            value.get(key),
            Some(&ResourceJsonValue::Number(expected.into()))
        );
    }
    assert_eq!(value.get("text").unwrap().as_str(), Some("1e2"));
    assert_eq!(value.get("negative_zero").unwrap().as_i64(), None);
    assert_eq!(
        value.get("precise").unwrap().as_i64(),
        Some(9007199254740993)
    );
    assert_eq!(value.get("exponent").unwrap().as_f64(), Some(100.0));
    assert_eq!(
        value.get("exponent").unwrap().to_nbt_summary().unwrap(),
        NbtSummaryValue::Double(100.0_f64.to_bits())
    );
    assert_eq!(
        value
            .get("negative_zero")
            .unwrap()
            .to_nbt_summary()
            .unwrap(),
        NbtSummaryValue::Double((-0.0_f64).to_bits())
    );
    assert_eq!(value.get("large").unwrap().as_u64(), None);
    assert_eq!(value.get("huge").unwrap().as_f64(), None);
    assert!(value.get("huge").unwrap().to_nbt_summary().is_err());
    assert_eq!(
        ResourceJsonValue::from_slice(b"1e400")
            .unwrap()
            .tree_size()
            .unwrap(),
        (1, 5)
    );
    let normalized: serde_json::Value = serde_json::from_str("1e2").unwrap();
    assert_eq!(
        ResourceJsonValue::from(&normalized),
        ResourceJsonValue::Number("100.0".into())
    );
}

#[test]
fn resource_json_rejects_invalid_source_and_json_syntax() {
    for bytes in [
        b"\"\xff\"".as_slice(),
        b"\"\xed\xa0\x80\"",
        b"{\"\xed\xa0\x80\":0}",
        br#""\uD80""#,
        br#""\uD80z""#,
        br#""\x41""#,
        br#"{"a":1,}"#,
        br#"[1,]"#,
        b"null true",
        b"01",
        b"\"line\nfeed\"",
        b"",
    ] {
        assert!(ResourceJsonValue::from_slice(bytes).is_err(), "{bytes:?}");
    }
}

#[test]
fn resource_json_depth_limit_spans_arrays_and_objects() {
    fn nested(depth: usize) -> String {
        let mut source = String::new();
        for level in 0..depth {
            source.push_str(if level % 2 == 0 { "[" } else { "{\"x\":" });
        }
        source.push_str(r#""\uD800""#);
        for level in (0..depth).rev() {
            source.push(if level % 2 == 0 { ']' } else { '}' });
        }
        source
    }
    let boundary = nested(MAX_RESOURCE_JSON_DEPTH);
    assert!(ResourceJsonValue::from_slice(boundary.as_bytes()).is_ok());
    let too_deep = nested(MAX_RESOURCE_JSON_DEPTH + 1);
    assert!(ResourceJsonValue::from_slice(too_deep.as_bytes()).is_err());
    let replaced = format!(r#"{{"x":{too_deep},"x":0}}"#);
    assert!(ResourceJsonValue::from_slice(replaced.as_bytes()).is_err());
}

#[test]
fn resource_json_tags_keep_unpaired_string_units_and_refuse_such_keys() {
    let value = ResourceJsonValue::from_slice(
        br#"{"name":"a\uD800","count":3,"list":[1.5,true],"nested":{"k":"v"}}"#,
    )
    .unwrap()
    .to_nbt_summary()
    .unwrap();
    let NbtSummaryValue::Compound(entries) = &value else {
        panic!("object should build a compound tag");
    };
    assert_eq!(entries[0].name, "name");
    assert_eq!(
        entries[0].value,
        NbtSummaryValue::string_from_utf16("a".encode_utf16().chain([0xd800]).collect())
    );
    assert_eq!(entries[1].value, NbtSummaryValue::Int(3));
    assert_eq!(
        entries[2].value,
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Double(1.5_f64.to_bits()),
            NbtSummaryValue::Byte(1),
        ])
    );

    // A tag name is a `String`, which is the bound the map-key ledger row owns.
    assert!(ResourceJsonValue::from_slice(br#"{"\uD800":1}"#)
        .unwrap()
        .to_nbt_summary()
        .is_err());
    assert!(ResourceJsonValue::from_slice(b"null")
        .unwrap()
        .to_nbt_summary()
        .is_err());
}
