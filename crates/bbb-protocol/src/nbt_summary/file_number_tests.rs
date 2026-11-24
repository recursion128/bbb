use super::*;

#[test]
fn named_file_numbers_canonicalize_nan_while_network_numbers_keep_raw_bits() {
    for bits in [
        0xffc0_1234,
        0x7f80_0001,
        f32::INFINITY.to_bits(),
        (-0.0_f32).to_bits(),
    ] {
        let value = NbtSummaryValue::Float(bits);
        let mut network = vec![5];
        network.extend_from_slice(&bits.to_be_bytes());
        assert_eq!(encode_nbt_summary(&value, 5).unwrap(), network);
        let canonical = if f32::from_bits(bits).is_nan() {
            f32::NAN.to_bits()
        } else {
            bits
        };
        let mut file = vec![5, 0, 0];
        file.extend_from_slice(&canonical.to_be_bytes());
        assert_eq!(encode_named_nbt_summary("", &value, 7).unwrap(), file);
        assert!(encode_named_nbt_summary("", &value, 6).is_err());
    }
    for bits in [
        0xfff8_0000_0000_1234,
        0x7ff0_0000_0000_0001,
        f64::NEG_INFINITY.to_bits(),
        (-0.0_f64).to_bits(),
    ] {
        let value = NbtSummaryValue::Double(bits);
        let mut network = vec![6];
        network.extend_from_slice(&bits.to_be_bytes());
        assert_eq!(encode_nbt_summary(&value, 9).unwrap(), network);
        let canonical = if f64::from_bits(bits).is_nan() {
            f64::NAN.to_bits()
        } else {
            bits
        };
        let mut file = vec![6, 0, 0];
        file.extend_from_slice(&canonical.to_be_bytes());
        assert_eq!(encode_named_nbt_summary("", &value, 11).unwrap(), file);
        assert!(encode_named_nbt_summary("", &value, 10).is_err());
    }
}

#[test]
fn named_file_nan_encoding_reaches_nested_compounds_and_lists() {
    let tree = |float, double| {
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "nested".into(),
            value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "float".into(),
                    value: NbtSummaryValue::Float(float),
                },
                NbtSummaryEntry {
                    name: "double".into(),
                    value: NbtSummaryValue::Double(double),
                },
            ])]),
        }])
    };
    let raw = tree(0xffc0_1234, 0xfff8_0000_0000_1234);
    let network = encode_nbt_summary(&raw, 1024).unwrap();
    assert_eq!(decode_nbt_summary(&network).unwrap(), Some(raw.clone()));
    let file = encode_named_nbt_summary("root", &raw, 1024).unwrap();
    let decoded = decode_file_saved_nbt_summary(&file).unwrap().unwrap();
    assert_eq!(decoded, tree(f32::NAN.to_bits(), f64::NAN.to_bits()));
    assert_eq!(
        encode_named_nbt_summary_java_string(&JavaString::from("root"), &raw, 1024).unwrap(),
        file,
    );
}
