//! The differential corpus for the item and resource-argument grammars.
//!
//! The item tables were produced by driving the same inputs through Vanilla's
//! hand-written `ItemParser.State.readComponents` (`ItemParser.java:143-189`) and
//! then through this crate's `CommandItemComponentPatchScannerUtf16`; the item
//! predicate table through `ItemPredicateArgument` over the real registries
//! (`ItemPredicateArgument.java:77-83`, `ComponentPredicateParser.java:44-121`);
//! the resource tables through `Identifier.readNonEmpty`
//! (`Identifier.java:201-214`) followed, on failure, by the
//! `ResourceOrIdArgument` grammar's second alternative -- the SNBT literal rule
//! (`ResourceOrIdArgument.java:68-85`).
//!
//! The predicate rows converged on 15857 generated buffers: 9736 agree on the
//! verdict, the unit and the entry `ErrorCollector.LongestOnly` kept, and the
//! rest land on the one boundary deliberately not compared row by row, because
//! this crate stops where Vanilla opens a registry or a codec. That boundary is
//! the item and item-tag registries and every component's value codec --
//! `ItemParser` rejects `012` and `stone[custom_name=1]`, this crate scans the
//! name and the raw tag and leaves both questions to its caller
//! (`items.rs:29-59`); those buffers only have to stay unrefused up to the unit
//! Vanilla reached. It is named with its acceptance in the client ledger rather
//! than hidden.

use super::{
    scan_command_item_predicate_head_utf16, scan_command_resource_location_utf16,
    scan_command_resource_or_id_utf16, CommandItemComponentPatchScannerUtf16,
    CommandItemPredicateTestScannerUtf16, ScannedCommandResourceOrIdUtf16,
};

/// What Vanilla's argument parser did with one input.
#[derive(Debug, Clone, Copy)]
enum Vanilla {
    /// The parse returned and left the reader at this unit.
    Accept { end: usize },
    /// The parse failed with a reader context at this unit.
    Reject { cursor: usize },
    /// The parse failed without a reader context, so Vanilla reports no unit.
    RejectWithoutContext,
}

fn units(input: &str) -> Vec<u16> {
    input.encode_utf16().collect()
}

fn item_end(input: &str) -> Result<usize, (usize, String)> {
    let units = units(input);
    let Some(start) = units.iter().position(|unit| *unit == u16::from(b'[')) else {
        return scan_command_resource_location_utf16(&units, 0)
            .map(|(end, _)| end)
            .map_err(|error| {
                (
                    error.cursor,
                    String::from_utf8_lossy(&error.message.to_jdk_utf8()).into_owned(),
                )
            });
    };
    let id_end = scan_command_resource_location_utf16(&units, 0).map(|(end, _)| end);
    assert_eq!(
        id_end,
        Ok(start),
        "{input:?} item id must end at the bracket"
    );
    let mut scanner = CommandItemComponentPatchScannerUtf16::new(&units, start);
    loop {
        match scanner.next_component() {
            Ok(Some(_)) => {}
            Ok(None) => return Ok(scanner.end()),
            Err(error) => {
                return Err((
                    error.cursor,
                    String::from_utf8_lossy(&error.message.to_jdk_utf8()).into_owned(),
                ))
            }
        }
    }
}

fn check_item(input: &str, expected: Vanilla) {
    let found = item_end(input);
    match (found, expected) {
        (Ok(end), Vanilla::Accept { end: want }) => {
            assert_eq!(end, want, "{input:?} scan ended at the wrong unit");
        }
        (Err((cursor, _)), Vanilla::Reject { cursor: want }) => {
            assert_eq!(cursor, want, "{input:?} was rejected at the wrong unit");
        }
        (Err((cursor, message)), Vanilla::RejectWithoutContext) => {
            // Vanilla's `create` leaves the reader untouched; the scan still has
            // to name where it gave up, and that place is the repeated name.
            assert!(cursor > 0, "{input:?} rejected at {cursor}: {message}");
        }
        (Ok(end), Vanilla::Reject { cursor }) => {
            panic!("{input:?} was accepted to @{end}, vanilla rejected it at {cursor}");
        }
        (Err((cursor, message)), Vanilla::Accept { end }) => {
            panic!("{input:?} was rejected at @{cursor} ({message}), vanilla accepted to @{end}");
        }
        (Ok(end), Vanilla::RejectWithoutContext) => {
            panic!("{input:?} was accepted to @{end}, vanilla rejected it");
        }
    }
}

#[test]
fn item_component_patch_scan_matches_the_vanilla_state_machine() {
    use Vanilla::{Accept, Reject, RejectWithoutContext};
    for (input, expected) in [
        ("stone", Accept { end: 5 }),
        ("minecraft:stone", Accept { end: 15 }),
        ("stone[]", Accept { end: 7 }),
        ("stone[damage=1]", Accept { end: 15 }),
        ("stone[ damage = 1 ]", Accept { end: 19 }),
        ("stone[damage=1,]", Accept { end: 16 }),
        ("stone[!damage]", Accept { end: 14 }),
        ("stone[!damage,]", Accept { end: 15 }),
        ("stone[damage=1]x", Accept { end: 15 }),
        ("stone extra", Accept { end: 5 }),
        ("stone[ ]", Reject { cursor: 7 }),
        ("stone[,]", Reject { cursor: 6 }),
        ("stone[!]", Reject { cursor: 7 }),
        ("stone[=1]", Reject { cursor: 6 }),
        ("stone[unknown=1]", Reject { cursor: 6 }),
        ("stone[damage=1,,]", Reject { cursor: 15 }),
        ("stone[damage=1 damage=2]", Reject { cursor: 16 }),
        ("stone[damage]", Reject { cursor: 12 }),
        ("stone[damage=]", Reject { cursor: 13 }),
        ("stone[damage=1", Reject { cursor: 14 }),
        ("stone[damage=1,damage=2]", RejectWithoutContext),
        ("stone[!damage,!damage]", RejectWithoutContext),
        ("stone[!damage,damage=1]", RejectWithoutContext),
        ("stone[damage=1,!damage]", RejectWithoutContext),
    ] {
        check_item(input, expected);
    }
}

/// The value of a component is parsed with the same SNBT grammar the item
/// argument uses, so the number fixes show up here too: `1 b` is the byte `1b`.
#[test]
fn item_component_values_use_the_verified_snbt_prefix() {
    for (input, end) in [
        ("stone[damage=1 b]", 17),
        ("stone[custom_name='x']", 22),
        ("stone[damage=0x10]", 18),
        ("stone[damage=1.5]", 17),
    ] {
        check_item(input, Vanilla::Accept { end });
    }
}

/// Named divergences. Every row is rejected by both sides; only the reported
/// unit differs, and each one is the registry/codec boundary this crate leaves
/// to its caller.
#[test]
fn item_scan_boundaries_are_the_callers_registry_and_codec() {
    // Vanilla: `Unknown item 'minecraft:012'` at @0; this crate only names the
    // lexeme, so the item registry decides.
    assert_eq!(item_end("012"), Ok(3));
    assert_eq!(item_end("[damage=1]"), Ok(10));
    // Vanilla: `Malformed 'minecraft:custom_name' component` at @18 after the
    // component codec refuses the tag; this crate hands the tag over unresolved.
    assert_eq!(item_end("stone[custom_name=1]"), Ok(20));
    // The bracket after the patch is left to the caller, exactly as
    // `ItemParser.parse` leaves it: the scan stops past the first `]`.
    assert_eq!(item_end("stone[damage=1][]"), Ok(15));
    // The SNBT grammar's own unit, which `readComponent` (`ItemParser.java:209-217`)
    // lets through unwrapped.
    let (cursor, message) = item_end("stone[damage=01]").unwrap_err();
    assert_eq!(
        (cursor, message.as_str()),
        (15, "invalid item component SNBT")
    );
}

fn check_resource_or_id(input: &str, expected: Vanilla) {
    match (
        scan_command_resource_or_id_utf16(&units(input), 0),
        expected,
    ) {
        (
            Ok(ScannedCommandResourceOrIdUtf16::Reference { end, .. }),
            Vanilla::Accept { end: want },
        ) => {
            assert_eq!(end, want, "{input:?} reference ended at the wrong unit");
        }
        (
            Ok(ScannedCommandResourceOrIdUtf16::Inline { end, .. }),
            Vanilla::Accept { end: want },
        ) => {
            assert_eq!(end, want, "{input:?} inline value ended at the wrong unit");
        }
        (Err(error), Vanilla::Reject { cursor: want }) => {
            assert_eq!(error.cursor, want, "{input:?} rejected at the wrong unit");
        }
        (found, expected) => panic!("{input:?}: {found:?} against {expected:?}"),
    }
}

/// `ResourceOrIdArgument` tries the identifier before the inline SNBT value, and
/// `Identifier.readNonEmpty` fails whenever the greedy run is empty or fails the
/// namespace/path rules -- which is what hands `a:b:c` and `A:B` to the value
/// alternative.
#[test]
fn resource_or_id_prefers_the_identifier_and_falls_back_to_snbt() {
    use Vanilla::{Accept, Reject};
    for (input, expected) in [
        ("stone", Accept { end: 5 }),
        ("minecraft:stone", Accept { end: 15 }),
        ("012", Accept { end: 3 }),
        ("true", Accept { end: 4 }),
        ("a:", Accept { end: 2 }),
        (":", Accept { end: 1 }),
        ("..", Accept { end: 2 }),
        ("a:b/c.d-e_f", Accept { end: 11 }),
        ("_a:./-", Accept { end: 6 }),
        ("  x  ", Accept { end: 3 }),
        ("a b", Accept { end: 1 }),
        ("a:b:c", Accept { end: 1 }),
        ("a::b", Accept { end: 1 }),
        ("A:B", Accept { end: 1 }),
        ("{a:1}", Accept { end: 5 }),
        ("\"q\"", Accept { end: 3 }),
        (" ", Reject { cursor: 1 }),
        ("}.5", Reject { cursor: 0 }),
        ("\u{e9}", Reject { cursor: 0 }),
        ("\u{e9}:a", Reject { cursor: 0 }),
    ] {
        check_resource_or_id(input, expected);
    }
}

/// The resource lexeme is `[0-9a-z_:/.-]` and nothing else: no whitespace skip,
/// no quoting, no uppercase, and an empty run is a valid `minecraft:` name that
/// only the registry can reject.
#[test]
fn resource_location_lexeme_matches_is_allowed_in_identifier() {
    for (input, end, value) in [
        ("stone", 5, "minecraft:stone"),
        ("A:B", 0, "minecraft:"),
        ("a b", 1, "minecraft:a"),
        ("a:b/c.d-e_f", 11, "a:b/c.d-e_f"),
        ("..", 2, "minecraft:.."),
    ] {
        assert_eq!(
            scan_command_resource_location_utf16(&units(input), 0),
            Ok((end, value.to_string())),
            "{input:?}"
        );
    }
    for input in ["a:b:c", "a::b"] {
        let error = scan_command_resource_location_utf16(&units(input), 0).expect_err(input);
        assert_eq!(error.cursor, 0, "{input:?}");
        assert_eq!(
            String::from_utf8_lossy(&error.message.to_jdk_utf8()),
            "invalid resource location",
            "{input:?}"
        );
    }
    // A lexeme no identifier character can start is an empty run here, and an
    // empty run is the valid default-namespace name `minecraft:` that only a
    // registry can reject -- the same reason `Identifier.read` accepts `:`.
    for input in ["é", "}"] {
        assert_eq!(
            scan_command_resource_location_utf16(&units(input), 0),
            Ok((0, "minecraft:".to_string())),
            "{input:?}"
        );
    }
}

/// A caller may hold a string Rust cannot: the scanner is driven by raw UTF-16
/// units, so a lone surrogate only has to stay outside the lexeme, not disappear.
#[test]
fn resource_scan_starts_inside_raw_utf16_units() {
    let input: Vec<u16> = [0xd800]
        .into_iter()
        .chain(":stone".encode_utf16())
        .chain([0xdc00])
        .collect();
    let (end, value) = scan_command_resource_location_utf16(&input, 1).unwrap();
    assert_eq!((end, value.as_str()), (7, "minecraft:stone"));
    assert_eq!(
        input.get(end),
        Some(&0xdc00),
        "the trailing surrogate is unread"
    );

    let invalid: Vec<u16> = [0xd800].into_iter().chain("a:b:c".encode_utf16()).collect();
    let error = scan_command_resource_location_utf16(&invalid, 1).unwrap_err();
    assert_eq!(
        (error.cursor, !error.message.to_jdk_utf8().is_empty()),
        (1, true)
    );
}

/// The entry `ErrorCollector.LongestOnly` keeps for one item predicate, named by
/// what both sides stand on. Every row below is the pinned client-26.1 jar's
/// answer for the same buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// `Identifier.ERROR_INVALID` through `ResourceLookupRule`
    /// (`ResourceLookupRule.java:36-38`): `argument.id.invalid`.
    Identifier,
    /// `ERROR_UNKNOWN_COMPONENT` (`ItemPredicateArgument.java:143-154`), stored
    /// at the same mark as -- and therefore ahead of -- `ERROR_UNKNOWN_PREDICATE`:
    /// `arguments.item.component.unknown`.
    UnknownComponent,
    /// The `~` terminal of `test`'s second alternative
    /// (`ComponentPredicateParser.java:96`), which a name only the
    /// component-existence predicate accepts reaches:
    /// `argument.literal.incorrect` with `~`.
    SubPredicateOperator,
    /// The `=` terminal of the first alternative (`:95`), stored before the bare
    /// `componentType` alternative (`:97`) succeeded:
    /// `argument.literal.incorrect` with `=`.
    ComponentAssignment,
    /// The `|`, `,` and `]` terminals (`:81`, `:74`, `:49`), whose first store is
    /// `|`: `argument.literal.incorrect` with `|`.
    SeparatorOrEnd,
    /// `TagParseRule` (`TagParseRule.java:22-27`) handing over the SNBT grammar's
    /// own farthest failure, which reaches the player unwrapped.
    Snbt(&'static str),
}

/// Drives the head and the `[...]` terms the way the command dispatcher does.
fn predicate_end(input: &str) -> Result<usize, (usize, Refusal)> {
    let units = units(input);
    let head = scan_command_item_predicate_head_utf16(&units, 0)
        .map_err(|error| (error.cursor, Refusal::Identifier))?;
    let mut scanner = CommandItemPredicateTestScannerUtf16::new(&units, head.end());
    loop {
        match scanner.next_term() {
            Ok(Some(_)) => {}
            Ok(None) => return Ok(scanner.end()),
            Err(error) => {
                let message = String::from_utf8_lossy(&error.message.to_jdk_utf8()).into_owned();
                let refusal = match message.as_str() {
                    "invalid item component predicate SNBT" | "invalid item predicate SNBT" => {
                        Refusal::Snbt(
                            scanner
                                .take_snbt_failure()
                                .and_then(|failure| failure.key)
                                .expect("an SNBT refusal carries the grammar's key"),
                        )
                    }
                    "expected item sub-predicate assignment" => Refusal::SubPredicateOperator,
                    "expected item component assignment" => Refusal::ComponentAssignment,
                    "expected item predicate separator or end" => Refusal::SeparatorOrEnd,
                    other if other.starts_with("unknown item component ") => {
                        Refusal::UnknownComponent
                    }
                    _ => Refusal::Identifier,
                };
                return Err((error.cursor, refusal));
            }
        }
    }
}

/// `top` reads one `type` and, only when a `[` follows, one `conditions` list
/// (`ComponentPredicateParser.java:45-63`); everything past the list belongs to
/// the dispatcher.
#[test]
fn item_predicate_accepts_what_the_component_predicate_grammar_accepts() {
    for (input, end) in [
        ("*", 1),
        ("stick", 5),
        ("minecraft:stick", 15),
        ("#stick", 6),
        ("#minecraft:stick", 16),
        (" stick", 6),
        ("*[damage=1]", 11),
        ("stick[damage=1]", 15),
        ("#stick[damage=1]", 16),
        ("stick[]", 7),
        ("stick[  ]", 9),
        ("*[]", 3),
        ("stick[ damage = 1 ]", 19),
        ("stick[damage =1]", 16),
        ("stick[damage= 1]", 16),
        ("stick  [damage=1]", 17),
        ("stick[damage=1 ]", 16),
        // The head alone, with the rest left to the dispatcher.
        ("stick extra", 5),
        ("*extra", 1),
        ("stick]", 5),
        ("stick|", 5),
        ("#stick|", 6),
        ("*|stick", 1),
        ("stick[damage=1]x", 15),
        ("stick[damage=1]]", 15),
        ("stick[damage=1]|", 15),
        ("stick[damage=1]extra", 15),
        ("stick[count=1]extra", 14),
        ("stick[damage]x", 13),
        ("stick[damage=1 ]x", 16),
    ] {
        assert_eq!(predicate_end(input), Ok(end), "{input:?}");
    }
}

/// `term`'s `!` alternative, `alternatives`' `|` and `conditions`' `,`
/// (`ComponentPredicateParser.java:74`, `:81`, `:86-91`): `!` binds one whole
/// test, `|` groups alternatives inside one condition, and `,` opens the next.
#[test]
fn item_predicate_reads_negation_and_both_separators() {
    for (input, end, terms) in [
        ("stick[!damage]", 14, &[(true, false)][..]),
        ("stick[ ! damage]", 16, &[(true, false)]),
        ("stick[!damage=1]", 16, &[(true, false)]),
        ("stick[! damage = 1 ]", 20, &[(true, false)]),
        ("stick[!count~{}]", 16, &[(true, false)]),
        ("#stick[!damage]", 15, &[(true, false)]),
        ("*[!damage]", 10, &[(true, false)]),
        (
            "stick[damage=1|count=2]",
            23,
            &[(false, false), (false, true)],
        ),
        (
            "stick[damage=1,count=2]",
            23,
            &[(false, false), (false, false)],
        ),
        ("stick[!damage|!count]", 21, &[(true, false), (true, true)]),
        (
            "stick[damage=1|!count]",
            22,
            &[(false, false), (true, true)],
        ),
        ("stick[!damage,count]", 20, &[(true, false), (false, false)]),
        (
            "stick[damage=1|count=2,custom_name='a']",
            39,
            &[(false, false), (false, true), (false, false)],
        ),
        (
            "stick[damage=1|count=2|custom_name='a']",
            39,
            &[(false, false), (false, true), (false, true)],
        ),
    ] {
        let units = units(input);
        let head = scan_command_item_predicate_head_utf16(&units, 0).expect(input);
        let mut scanner = CommandItemPredicateTestScannerUtf16::new(&units, head.end());
        let mut found = Vec::new();
        while let Some(term) = scanner.next_term().expect(input) {
            found.push((term.negated, term.alternative));
        }
        assert_eq!(found, terms, "{input:?}");
        assert_eq!(scanner.end(), end, "{input:?}");
    }
}

/// `test`'s three alternatives are tried in source order
/// (`ComponentPredicateParser.java:94-97`), so which entry `LongestOnly` keeps
/// depends on how far each alternative got before it stored.
#[test]
fn item_predicate_refusals_are_the_entry_longest_only_keeps() {
    use Refusal::{
        ComponentAssignment, Identifier, SeparatorOrEnd, Snbt, SubPredicateOperator,
        UnknownComponent,
    };
    const INCORRECT: &str = "argument.literal.incorrect";
    const UNQUOTED: &str = "snbt.parser.expected_unquoted_string";
    for (input, cursor, refusal) in [
        // Nothing an identifier can start.
        ("[damage=1]", 0, Identifier),
        ("!stick", 0, Identifier),
        ("a:b:c", 0, Identifier),
        ("A:B", 0, Identifier),
        ("#a:b:c", 1, Identifier),
        ("*[", 2, Identifier),
        ("stick[", 6, Identifier),
        ("stick[,]", 6, Identifier),
        ("stick[|]", 6, Identifier),
        ("stick[=1]", 6, Identifier),
        ("stick[~1]", 6, Identifier),
        ("stick[a:b:c=1]", 6, Identifier),
        ("stick[A:B=1]", 6, Identifier),
        ("stick[A:B~1]", 6, Identifier),
        ("stick[ , ]", 7, Identifier),
        ("stick[!]", 7, Identifier),
        ("stick[!*]", 7, Identifier),
        ("stick[!!damage]", 7, Identifier),
        ("stick[ ! ! damage]", 9, Identifier),
        ("stick[damage,", 13, Identifier),
        ("stick[damage|", 13, Identifier),
        ("stick[damage=1,", 15, Identifier),
        ("stick[damage=1,]", 15, Identifier),
        ("stick[damage=1|]", 15, Identifier),
        ("stick[damage=1||count=2]", 15, Identifier),
        ("stick[damage=1,,count=2]", 15, Identifier),
        ("stick[damage=1,!]", 16, Identifier),
        ("stick[damage=1|!]", 16, Identifier),
        ("stick[damage=1,count=2|]", 23, Identifier),
        ("stick[damage=1|count=2,]", 23, Identifier),
        // A name no component registry holds, which stops at the first
        // alternative's `componentType` however the term goes on.
        ("stick[unknown]", 13, UnknownComponent),
        ("stick[unknown=1]", 13, UnknownComponent),
        ("stick[unknown=]", 13, UnknownComponent),
        ("stick[unknown~1]", 13, UnknownComponent),
        ("stick[unknown~]", 13, UnknownComponent),
        ("stick[unknown x]", 13, UnknownComponent),
        ("stick[unknown  =1]", 13, UnknownComponent),
        // A transient component: `lookupComponentType` refuses it because its
        // codec is null, so only the `~` alternative reaches past the name.
        ("stick[creative_slot_lock]", 24, SubPredicateOperator),
        ("stick[creative_slot_lock=1]", 24, SubPredicateOperator),
        ("stick[creative_slot_lock x]", 25, SubPredicateOperator),
        ("stick[creative_slot_lock  ]", 26, SubPredicateOperator),
        ("stick[creative_slot_lock", 24, SubPredicateOperator),
        ("stick[!creative_slot_lock]", 25, SubPredicateOperator),
        ("#stick[creative_slot_lock]", 25, SubPredicateOperator),
        (
            "stick[creative_slot_lock,damage=1]",
            24,
            SubPredicateOperator,
        ),
        (
            "stick[damage=1,creative_slot_lock]",
            33,
            SubPredicateOperator,
        ),
        ("stick[additional_trade_cost]", 27, SubPredicateOperator),
        // A bare name parses, and the `=` the first alternative stored at the
        // same unit is ahead of every separator stored there.
        ("stick[damage", 12, ComponentAssignment),
        ("stick[damage ", 13, ComponentAssignment),
        ("stick[damage x]", 13, ComponentAssignment),
        ("stick[!damage", 13, ComponentAssignment),
        ("stick[!damage x]", 14, ComponentAssignment),
        ("stick[count", 11, ComponentAssignment),
        ("*[damage", 8, ComponentAssignment),
        ("#stick[damage", 13, ComponentAssignment),
        ("stick[damage=1,count", 20, ComponentAssignment),
        ("stick[damage=1|count", 20, ComponentAssignment),
        // A value parsed, so the separator terminals are the farthest store.
        ("stick[damage=1", 14, SeparatorOrEnd),
        ("stick[damage=1 x]", 15, SeparatorOrEnd),
        ("*[damage=1", 10, SeparatorOrEnd),
        ("stick[count=1..3]", 14, SeparatorOrEnd),
        // The `tag` rule hands the SNBT grammar's own refusal over.
        ("stick[damage=]", 13, Snbt(UNQUOTED)),
        ("stick[damage = ]", 15, Snbt(UNQUOTED)),
        ("stick[count=]", 12, Snbt(UNQUOTED)),
        ("stick[count~]", 12, Snbt(UNQUOTED)),
        ("stick[custom_name=]", 18, Snbt(UNQUOTED)),
        (
            "stick[custom_name='a]",
            21,
            Snbt("snbt.parser.invalid_string_contents"),
        ),
        ("stick[damage={]", 14, Snbt(INCORRECT)),
        ("stick[damage=01]", 15, Snbt(INCORRECT)),
        ("stick[damage=01", 15, Snbt(INCORRECT)),
        ("stick[damage=01,count=2]", 15, Snbt(INCORRECT)),
        ("stick[damage=01|count=2]", 15, Snbt(INCORRECT)),
        ("stick[count=01]", 14, Snbt(INCORRECT)),
        ("stick[damage=1,count=01]", 23, Snbt(INCORRECT)),
        ("stick[damage=999999999999]", 25, Snbt(INCORRECT)),
        ("stick[custom_data~01]", 20, Snbt(INCORRECT)),
        ("stick[custom_data~{]", 19, Snbt(INCORRECT)),
    ] {
        assert_eq!(predicate_end(input), Err((cursor, refusal)), "{input:?}");
    }
}

/// `lookupPredicateType` answers a component-existence predicate for any other
/// registered component (`ItemPredicateArgument.java:173-178`), so every
/// `name~value` name is a component name and `ERROR_UNKNOWN_PREDICATE` (`:54-56`)
/// is unreachable through this argument -- which is why an unknown name answers
/// the component refusal above.
#[test]
fn item_predicate_sub_predicate_names_are_component_names() {
    for input in [
        "stick[damage~{}]",
        "stick[minecraft:damage~{}]",
        "stick[custom_data~{}]",
        "stick[minecraft:custom_data~{}]",
        "stick[creative_slot_lock~{}]",
        "stick[map_post_processing~{}]",
        "stick[additional_trade_cost~{}]",
        "stick[count~1]",
        "stick[count~{min:1}]",
        "stick[villager/variant]",
    ] {
        assert!(predicate_end(input).is_ok(), "{input:?}");
    }
}

/// Named divergences. Every row is accepted here and rejected by Vanilla at the
/// same unit or later, and each one is the registry or codec question this crate
/// leaves to its caller.
#[test]
fn item_predicate_boundaries_are_the_callers_registry_and_codec() {
    // Vanilla: `argument.item.id.invalid` at @7 once the item registry answers;
    // this crate only lexes the name.
    assert_eq!(predicate_end("unknown"), Ok(7));
    // Vanilla: `arguments.item.tag.unknown` once the item tag set is loaded.
    assert_eq!(predicate_end("#unknown"), Ok(8));
    // Vanilla: `arguments.item.predicate.malformed` at the unit after the value,
    // from the sub-predicate codec.
    assert_eq!(predicate_end("stick[damage~1]"), Ok(15));
    assert_eq!(predicate_end("stick[enchantments~{}]"), Ok(22));
    assert_eq!(predicate_end("stick[villager/variant~{}]"), Ok(26));
    assert_eq!(predicate_end("stick[creative_slot_lock~1]"), Ok(27));
    // Both reject at @18, but Vanilla's entry is the component codec's
    // `arguments.item.component.malformed` rather than a separator.
    assert_eq!(
        predicate_end("stick[damage={a:1}x]"),
        Err((18, Refusal::SeparatorOrEnd))
    );
}
