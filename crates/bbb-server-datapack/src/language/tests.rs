use super::*;

fn text(value: &NbtSummaryValue) -> String {
    String::from_utf16(&component_plain_text(value)).expect("the fixture text is valid UTF-16")
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::string(value)
}

fn translatable(key: &str, args: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    let mut entries = vec![entry("translate", string(key))];
    if !args.is_empty() {
        entries.push(entry("with", NbtSummaryValue::List(args)));
    }
    NbtSummaryValue::Compound(entries)
}

#[test]
fn the_default_language_applies_the_deprecated_renames_and_removals() {
    let language = ServerLanguage::get();
    assert_eq!(
        language.get_or_default("entity.minecraft.zombie", "?"),
        "Zombie"
    );
    assert!(!language.has("upgrade.minecraft.netherite_upgrade"));
    assert!(!language.has("item.minecraft.dune_armor_trim_smithing_template.new"));
    assert!(language.has("item.minecraft.dune_armor_trim_smithing_template"));
    assert_eq!(
        language.get_or_default("no.such.key", "no.such.key"),
        "no.such.key"
    );
}

#[test]
fn load_rewrites_the_unsupported_numeric_formats_and_resolves_renames() {
    let language = ServerLanguage::load(
        r#"{"a": "%d and %2$.2f", "b": "kept", "c": "renamed", "d": "gone", "\ud800": "%d\udc00"}"#,
        r#"{"removed": ["d"], "renamed": {"c": "e", "missing": "b"}}"#,
    )
    .expect("the documents load");
    assert_eq!(language.get_or_default("a", "?"), "%s and %2$s");
    assert_eq!(
        language.storage.get(&[0xd800][..]).unwrap().units(),
        &[u16::from(b'%'), u16::from(b's'), 0xdc00],
    );
    // A rename whose source is absent removes its target
    // (DeprecatedTranslationsInfo.java:53-58).
    assert!(!language.has("b"));
    assert_eq!(language.get_or_default("e", "?"), "renamed");
    assert!(!language.has("c") && !language.has("d"));
}

#[test]
fn get_string_walks_contents_then_siblings() {
    assert_eq!(text(&string("plain")), "plain");
    let component = NbtSummaryValue::Compound(vec![
        entry("text", string("A ")),
        entry("color", string("red")),
        entry(
            "extra",
            NbtSummaryValue::List(vec![
                translatable("entity.minecraft.zombie", Vec::new()),
                NbtSummaryValue::Compound(vec![entry("keybind", string("key.jump"))]),
                NbtSummaryValue::Compound(vec![entry("selector", string("@p"))]),
                NbtSummaryValue::Compound(vec![entry(
                    "score",
                    NbtSummaryValue::Compound(vec![
                        entry("name", string("*")),
                        entry("objective", string("o")),
                    ]),
                )]),
            ]),
        ),
    ]);
    assert_eq!(text(&component), "A Zombiekey.jump@p");
    assert_eq!(
        text(&NbtSummaryValue::List(vec![string("x"), string("y")])),
        "xy"
    );
    let object = |fallback: Option<NbtSummaryValue>| {
        let mut entries = vec![
            entry("type", string("object")),
            entry("sprite", string("item/apple")),
        ];
        if let Some(fallback) = fallback {
            entries.push(entry("fallback", fallback));
        }
        NbtSummaryValue::Compound(entries)
    };
    assert_eq!(text(&object(Some(string("apple")))), "apple");
    assert_eq!(text(&object(None)), "[item/apple]");
    let info = |fields: Vec<NbtSummaryEntry>| {
        let mut entries = vec![entry("type", string("object"))];
        entries.extend(fields);
        text(&NbtSummaryValue::Compound(entries))
    };
    assert_eq!(
        info(vec![
            entry("sprite", string("minecraft:item/apple")),
            entry("atlas", string("minecraft:items")),
        ]),
        "[item/apple@items]"
    );
    assert_eq!(
        info(vec![
            entry("object", string("atlas")),
            entry("sprite", string("custom:thing")),
            entry("atlas", string("minecraft:blocks")),
        ]),
        "[custom:thing]"
    );
    assert_eq!(info(vec![entry("player", string("Steve"))]), "[Steve head]");
    assert_eq!(
        info(vec![
            entry("object", string("player")),
            entry("player", NbtSummaryValue::Compound(Vec::new())),
        ]),
        "[unknown player head]"
    );
}

#[test]
fn translations_decompose_the_way_translatable_contents_does() {
    let args = || vec![string("a"), string("b"), string("c"), string("d")];
    for (key, args, expected) in [
        (
            "translation.test.complex",
            args(),
            "Prefix, ab again b and a lastly c and also a again!",
        ),
        ("translation.test.escape", args(), "%s %a %%s %%b"),
        ("translation.test.invalid", args(), "hi %"),
        ("translation.test.invalid2", args(), "hi %  s"),
        // `getArgument` past the end fails the whole template.
        ("translation.test.args", vec![string("a")], "%s %s"),
        ("no.such.key", Vec::new(), "no.such.key"),
        (
            "chat.square_brackets",
            vec![NbtSummaryValue::Float(1.5f32.to_bits())],
            "[1.5]",
        ),
        (
            "chat.square_brackets",
            vec![NbtSummaryValue::Double(1.0e10f64.to_bits())],
            "[1.0E10]",
        ),
    ] {
        assert_eq!(
            String::from_utf16(&translatable_plain_text(key, &args)).expect("utf-16"),
            expected,
            "{key}"
        );
    }
    let with_fallback = NbtSummaryValue::Compound(vec![
        entry("translate", string("no.such.key")),
        entry("fallback", string("Fallback %s")),
        entry("with", NbtSummaryValue::List(vec![NbtSummaryValue::Int(7)])),
    ]);
    assert_eq!(text(&with_fallback), "Fallback 7");
    let raw = NbtSummaryValue::Compound(vec![
        entry(
            "translate",
            NbtSummaryValue::string_from_utf16_slice(&[0xd800]),
        ),
        entry(
            "fallback",
            NbtSummaryValue::string_from_utf16_slice(&[0xd800, 37, 115, 0xdc00]),
        ),
        entry(
            "with",
            NbtSummaryValue::List(vec![NbtSummaryValue::string_from_utf16_slice(&[0xd801])]),
        ),
    ]);
    assert_eq!(component_plain_text(&raw), [0xd800, 0xd801, 0xdc00]);
}

#[test]
fn translatable_expansion_bomb_is_bounded_in_time_and_output() {
    let repeat_ten = "%1$s".repeat(10);
    let build_bomb = |leaf: &str| {
        let mut node = string(leaf);
        for _ in 0..12 {
            node = translatable(&repeat_ten, vec![node]);
        }
        node
    };

    let single_char = component_plain_text(&build_bomb("X"));
    assert!(!single_char.is_empty() && single_char.len() <= MAX_PLAIN_TEXT_UNITS);
    assert!(single_char.iter().all(|&unit| unit == u16::from(b'X')));

    // A wider leaf exhausts the UTF-16 output unit budget before the step budget.
    let wide_leaf = "X".repeat(16);
    let wide = component_plain_text(&build_bomb(&wide_leaf));
    assert_eq!(wide.len(), MAX_PLAIN_TEXT_UNITS);
    assert!(wide.iter().all(|&unit| unit == u16::from(b'X')));

    // An empty-leaf bomb produces zero UTF-16 units and must still terminate via the step cap.
    let empty = component_plain_text(&build_bomb(""));
    assert!(empty.is_empty());

    // Direct entry via `translatable_plain_text` enforces the same shared budget.
    let direct = translatable_plain_text(&repeat_ten, &[build_bomb(&wide_leaf)]);
    assert_eq!(direct.len(), MAX_PLAIN_TEXT_UNITS);
}
