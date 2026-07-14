use super::*;

#[test]
fn accepts_component_root_alternatives_and_simple_contents() {
    for value in [
        string("literal"),
        list([string("first"), compound([("text", string("second"))])]),
        compound([("text", string("plain"))]),
        compound([
            ("type", string("text")),
            ("text", string("typed")),
            ("unknown", NbtSummaryValue::Int(1)),
        ]),
        compound([
            ("translate", string("chat.type.text")),
            ("fallback", NbtSummaryValue::Int(1)),
            (
                "with",
                list([
                    NbtSummaryValue::Int(42),
                    compound([("text", string("sender"))]),
                ]),
            ),
        ]),
        compound([("keybind", string("key.jump"))]),
        compound([(
            "score",
            compound([("name", string("@s")), ("objective", string("points"))]),
        )]),
    ] {
        validate_component_nbt(&value, "component").unwrap();
    }
}

#[test]
fn accepts_structural_complex_contents_extra_and_scalar_style() {
    let value = compound([
        ("selector", string("@a")),
        ("separator", string(", ")),
        (
            "extra",
            list([
                compound([
                    ("nbt", string("Pos[0]")),
                    ("source", string("storage")),
                    ("storage", string("example:data")),
                    ("interpret", string("leniently ignored")),
                ]),
                compound([
                    ("type", string("object")),
                    ("object", string("atlas")),
                    ("sprite", string("item/diamond")),
                    ("fallback", string("diamond")),
                ]),
            ]),
        ),
        ("color", string("dark_blue")),
        (
            "shadow_color",
            list([
                NbtSummaryValue::Float(0.0f32.to_bits()),
                NbtSummaryValue::Int(1),
                NbtSummaryValue::Double(0.5f64.to_bits()),
                NbtSummaryValue::Byte(1),
            ]),
        ),
        ("bold", NbtSummaryValue::Int(1)),
        ("insertion", string("insert")),
        ("font", string("uniform")),
    ]);
    validate_component_nbt(&value, "component").unwrap();
}

#[test]
fn rejects_invalid_roots_lists_dispatch_and_extra() {
    for (value, expected) in [
        (NbtSummaryValue::Int(1), "is not a component"),
        (NbtSummaryValue::List(Vec::new()), "component list is empty"),
        (NbtSummaryValue::Compound(Vec::new()), "no matching"),
        (
            compound([("type", NbtSummaryValue::Int(1))]),
            "type is not a string",
        ),
        (
            compound([("type", string("unknown"))]),
            "unknown component content type",
        ),
        (
            compound([
                ("type", string("text")),
                ("translate", string("fallback.must.not.win")),
            ]),
            "missing field text",
        ),
        (
            compound([
                ("text", string("value")),
                ("extra", NbtSummaryValue::List(Vec::new())),
            ]),
            "field extra is empty",
        ),
        (
            compound([("text", string("value")), ("extra", string("not a list"))]),
            "extra is not a list",
        ),
    ] {
        let error = validate_component_nbt(&value, "component").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

#[test]
fn rejects_malformed_simple_and_nested_content_fields() {
    for (value, expected) in [
        (
            compound([
                ("type", string("translatable")),
                ("translate", NbtSummaryValue::Int(1)),
            ]),
            "translate is not a string",
        ),
        (
            compound([("translate", string("key")), ("with", string("not a list"))]),
            "no matching",
        ),
        (
            compound([
                ("type", string("translatable")),
                ("translate", string("key")),
                ("with", list([NbtSummaryValue::List(Vec::new())])),
            ]),
            "component list is empty",
        ),
        (
            compound([("type", string("score")), ("score", string("wrong"))]),
            "score is not a compound",
        ),
        (
            compound([
                ("type", string("score")),
                ("score", compound([("name", string("player"))])),
            ]),
            "missing field objective",
        ),
        (
            compound([
                ("type", string("nbt")),
                ("nbt", string("Pos")),
                ("source", string("storage")),
                ("storage", string("Bad")),
            ]),
            "not a valid identifier",
        ),
        (
            compound([
                ("type", string("nbt")),
                ("nbt", string("Pos")),
                ("block", string("~ ~ ~")),
                ("interpret", NbtSummaryValue::Byte(1)),
                ("plain", NbtSummaryValue::Byte(1)),
            ]),
            "cannot both be enabled",
        ),
        (
            compound([
                ("type", string("object")),
                ("object", string("atlas")),
                ("sprite", string("Bad")),
            ]),
            "not a valid identifier",
        ),
    ] {
        let error = validate_component_nbt(&value, "component").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

#[test]
fn rejects_malformed_scalar_style_fields() {
    for (field, value, expected) in [
        ("color", string("#1234567"), "color is invalid"),
        ("bold", string("true"), "bold is not a boolean"),
        (
            "shadow_color",
            list([
                NbtSummaryValue::Int(1),
                NbtSummaryValue::Int(2),
                NbtSummaryValue::Int(3),
            ]),
            "four-number vector4f",
        ),
        (
            "insertion",
            NbtSummaryValue::Int(1),
            "insertion is not a string",
        ),
        ("font", string("Bad"), "font is not a valid identifier"),
    ] {
        let value =
            NbtSummaryValue::Compound(vec![entry("text", string("styled")), entry(field, value)]);
        let error = validate_component_nbt(&value, "component").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

fn compound<const N: usize>(entries: [(&str, NbtSummaryValue); N]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| entry(name, value))
            .collect(),
    )
}

fn list<const N: usize>(values: [NbtSummaryValue; N]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.into_iter().collect())
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}
