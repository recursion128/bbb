use super::*;
use crate::{
    packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
    },
    NbtSummaryValue,
};

#[test]
fn context_free_dispatch_owns_its_families_and_declines_the_rest() {
    let scanned = scan_command_context_free_argument(
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID,
        "survival",
        0,
        &CommandArgumentTemplate::Unit,
    )
    .expect("gamemode is a context-free family")
    .expect("survival is a game mode");
    assert_eq!(scanned, "survival".len());
    assert!(scan_command_context_free_argument(
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
        "minecraft:stone",
        0,
        &CommandArgumentTemplate::Unit,
    )
    .is_none());
}

#[test]
fn scanners_report_absolute_cursors_into_the_whole_command() {
    let command = "give @s minecraft:stone[minecraft:damage=1]";
    let start = "give @s ".len();
    let (end, item) = scan_command_resource_location(command, start).expect("item name");
    assert_eq!(item, "minecraft:stone");
    assert_eq!(end, start + "minecraft:stone".len());

    let mut patch = CommandItemComponentPatchScanner::new(command, end);
    let component = patch
        .next_component()
        .expect("component scan")
        .expect("one component");
    assert_eq!(component.start, end + 1);
    assert!(!component.removed);
    assert_eq!(component.component.resource_id, "minecraft:damage");
    assert_eq!(component.value, Some(NbtSummaryValue::Int(1)));
    assert!(patch.next_component().expect("patch end").is_none());
    assert_eq!(patch.end(), command.len());
}

#[test]
fn component_patch_stops_before_a_later_fault_so_a_caller_can_reject_the_first_value() {
    // Vanilla resolves each component the moment it has read it, so a patch
    // holding both a bad value and a later syntax fault reports the value. The
    // scanner has to hand back the first entry before it sees the fault.
    let command = "[minecraft:damage=1,minecraft:custom_name}";
    let mut patch = CommandItemComponentPatchScanner::new(command, 0);
    let first = patch
        .next_component()
        .expect("first entry")
        .expect("one component");
    assert_eq!(first.component.resource_id, "minecraft:damage");
    let fault = patch.next_component().expect_err("unterminated patch");
    assert_eq!(fault.kind, CommandSyntaxErrorKind::InvalidArgument);
}

#[test]
fn component_patch_rejects_repeats_and_names_that_are_not_persistent_components() {
    let mut repeated =
        CommandItemComponentPatchScanner::new("[minecraft:damage=1,minecraft:damage=2]", 0);
    repeated.next_component().expect("first entry");
    assert!(repeated.next_component().is_err());

    let mut unknown = CommandItemComponentPatchScanner::new("[minecraft:not_a_component=1]", 0);
    assert!(unknown.next_component().is_err());
}

#[test]
fn item_predicate_heads_and_tests_carry_the_cursor_their_errors_are_raised_at() {
    let command = "#minecraft:logs[minecraft:damage=1|minecraft:count~{min:1}]";
    let head = scan_command_item_predicate_head(command, 0).expect("tag head");
    let ScannedCommandItemPredicateHead::Tag {
        start, resource_id, ..
    } = &head
    else {
        panic!("expected a tag head, got {head:?}");
    };
    assert_eq!(*start, 0);
    assert_eq!(resource_id, "minecraft:logs");

    let mut tests = CommandItemPredicateTestScanner::new(command, head.end());
    let exact = tests.next_term().expect("first test").expect("a test");
    assert!(!exact.alternative);
    assert!(matches!(
        exact.test,
        ScannedCommandItemPredicateTest::Exact {
            component: Some(_),
            ..
        }
    ));
    let subpredicate = tests.next_term().expect("second test").expect("a test");
    assert!(subpredicate.alternative);
    assert!(matches!(
        subpredicate.test,
        ScannedCommandItemPredicateTest::Subpredicate { .. }
    ));
    assert!(tests.next_term().expect("test list end").is_none());
    assert_eq!(tests.end(), command.len());
}

#[test]
fn item_predicate_without_tests_consumes_only_its_head() {
    let mut tests = CommandItemPredicateTestScanner::new("*", 1);
    assert!(tests.next_term().expect("no tests").is_none());
    assert_eq!(tests.end(), 1);
}

#[test]
fn block_scan_splits_at_the_name_so_an_unknown_block_is_reported_before_its_properties() {
    let command = "minecraft:oak_log[axis=x]{Custom:1b}";
    let identifier = scan_command_block_identifier(command, 0, false).expect("block name");
    assert!(!identifier.tagged);
    assert_eq!(identifier.resource_id, "minecraft:oak_log");
    assert_eq!(identifier.end, "minecraft:oak_log".len());

    let body = scan_command_block_body(command, identifier.end).expect("block body");
    assert_eq!(body.properties.get("axis").map(String::as_str), Some("x"));
    assert_eq!(body.end, command.len());

    assert!(scan_command_block_identifier("#minecraft:logs", 0, false).is_err());
    assert!(scan_command_block_identifier("#minecraft:logs", 0, true).is_ok());
    assert_eq!(
        scan_command_block_body("minecraft:stone", "minecraft:stone".len()),
        Ok(ScannedCommandBlockBody {
            properties: std::collections::BTreeMap::new(),
            end: "minecraft:stone".len(),
        })
    );
}

#[test]
fn particle_options_scanner_leaves_required_fields_to_codec() {
    let scanned =
        scan_command_particle_options("{color:16711680,scale:1.0f}", 0).expect("options compound");
    assert_eq!(scanned.end, "{color:16711680,scale:1.0f}".len());
    assert_eq!(scanned.fields.as_ref().map(Vec::len), Some(2));

    let empty = scan_command_particle_options("", 0).expect("options may be absent");
    assert_eq!(empty, ScannedCommandParticleOptions::default());
}

#[test]
fn resource_or_id_prefers_a_name_and_falls_back_to_inline_snbt() {
    let reference = scan_command_resource_or_id("example:notice", 0).expect("a name");
    assert_eq!(
        reference,
        ScannedCommandResourceOrId::Reference {
            end: "example:notice".len(),
            resource_id: "example:notice".to_string(),
        }
    );
    let inline = scan_command_resource_or_id("{pools:[]}", 0).expect("inline SNBT");
    assert_eq!(inline.end(), "{pools:[]}".len());
    assert!(matches!(inline, ScannedCommandResourceOrId::Inline { .. }));
    assert!(scan_command_resource_or_id("!", 0).is_err());
}

#[test]
fn string_reader_primitives_keep_brigadier_character_classes() {
    assert_eq!(
        scan_command_unquoted_string("value+1 rest", 0),
        "value+1".len()
    );
    assert_eq!(scan_command_token("value+1 rest", 0), "value+1".len());
    assert_eq!(
        read_command_string_value(r#""a\"b" rest"#, 0),
        Ok((r#""a\"b""#.len(), "a\"b".to_string()))
    );
    assert_eq!(
        scan_command_quotable_string("'oops", 0)
            .expect_err("unterminated quote")
            .kind,
        CommandSyntaxErrorKind::ExpectedQuoteEnd
    );
    assert_eq!(scan_command_number_lexeme("-1.5x", 0), Ok((4, "-1.5")));
    assert_eq!(
        scan_command_number_lexeme("x", 0)
            .expect_err("no number")
            .kind,
        CommandSyntaxErrorKind::ExpectedNumber
    );
}

#[test]
fn resource_selector_scans_a_namespaced_pattern_and_matches_glob_shapes() {
    assert_eq!(
        scan_command_resource_selector("st?ne rest", 0),
        Ok(("st?ne".len(), "minecraft:st?ne".to_string()))
    );
    assert!(command_resource_selector_matches(
        "minecraft:st?ne",
        "minecraft:stone"
    ));
    assert!(!command_resource_selector_matches(
        "minecraft:*",
        "example:stone"
    ));
    assert_eq!(
        command_stat_key_for_resource_location("minecraft:mined"),
        "minecraft.mined"
    );
}
