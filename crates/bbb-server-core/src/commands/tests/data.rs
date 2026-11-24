//! `/data` reads and mutations against native accessors.
use super::*;

#[test]
fn storage_get_reports_the_scaled_value_and_merge_keeps_existing_fields() {
    let mut fixture = CommandTestWorld::default();
    fixture.setup("data merge storage demo:state {answer:21}");
    let query = ran("data get storage demo:state answer 2", &mut fixture);
    assert_eq!(query.result, Some(42));
    assert_eq!(keys(&query), ["commands.data.storage.get"]);
    let merge = ran("data merge storage demo:state {after:2}", &mut fixture);
    assert_eq!(merge.result, Some(1));
    assert_eq!(keys(&merge), ["commands.data.storage.modified"]);
    assert_eq!(
        fixture.storage("demo:state"),
        compound(vec![
            ("answer", NbtSummaryValue::Int(21)),
            ("after", NbtSummaryValue::Int(2))
        ])
    );
}

#[test]
fn modify_reads_a_second_native_storage_accessor() {
    let mut fixture = CommandTestWorld::default();
    fixture.setup("data merge storage demo:source {value:[1,2]}");
    fixture.setup("data merge storage demo:target {before:7}");
    let run = ran(
        "data modify storage demo:target copied set from storage demo:source value",
        &mut fixture,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.data.storage.modified"]);
    assert_eq!(
        fixture.storage("demo:target"),
        compound(vec![
            ("before", NbtSummaryValue::Int(7)),
            (
                "copied",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(1), NbtSummaryValue::Int(2)])
            )
        ])
    );
    fixture.setup("data merge storage demo:source {text:'😀'}");
    let sliced = ran(
        "data modify storage demo:target raw set string storage demo:source text 0 1",
        &mut fixture,
    );
    assert_eq!(sliced.result, Some(1));
    let NbtSummaryValue::Compound(fields) = fixture.storage("demo:target") else {
        panic!("storage compound")
    };
    assert_eq!(
        fields
            .iter()
            .find(|field| field.name == "raw")
            .map(|field| &field.value),
        Some(&NbtSummaryValue::string_from_utf16_slice(&[0xd83d]))
    );
    let length = ran("data get storage demo:target raw", &mut fixture);
    assert_eq!(length.result, Some(1));
}

#[test]
fn entity_query_allows_players_but_entity_mutation_refuses_them() {
    let mut fixture = world(&[user("alice", 1)]);
    let query = ran("data get entity alice Health", &mut fixture);
    assert_eq!(query.result, Some(20));
    assert_eq!(keys(&query), ["commands.data.entity.query"]);
    let mutation = ran("data merge entity alice {Health:19f}", &mut fixture);
    assert_eq!(mutation.result, None);
    assert_eq!(keys(&mutation), ["commands.data.entity.invalid"]);
}

#[test]
fn stored_entity_mutation_reaches_the_selected_level() {
    let entities = [stored_entity(
        9,
        44,
        [0.0, 64.0, 0.0],
        crate::entity::EntityData::Marker(crate::entity::MarkerData::default()),
    )];
    let mut fixture =
        world(&[]).with_entities_in(level_key(Java26BuiltinDimension::End), &entities);
    let modified = ran(
        "data merge entity 0-0-0-0-9 {data:{answer:2}}",
        &mut fixture,
    );
    assert_eq!(modified.result, Some(1));
    assert_eq!(keys(&modified), ["commands.data.entity.modified"]);
    let query = ran("data get entity 0-0-0-0-9 data.answer", &mut fixture);
    assert_eq!(query.result, Some(2));
    assert_eq!(keys(&query), ["commands.data.entity.query"]);
}

#[test]
fn block_accessor_mutates_the_source_level_block_entity() {
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let mut fixture = CommandTestWorld::default()
        .with_level(level_key(Java26BuiltinDimension::Nether))
        .with_chest(pos);
    let modified = ran(
        "data merge block 1 64 1 {CustomName:'\"Nether chest\"'}",
        &mut fixture,
    );
    assert_eq!(modified.result, Some(1));
    assert_eq!(keys(&modified), ["commands.data.block.modified"]);
    let query = ran("data get block 1 64 1 CustomName", &mut fixture);
    assert_eq!(keys(&query), ["commands.data.block.query"]);
    assert!(query.result.is_some());
}

#[test]
fn storage_argument_suggestions_use_existing_native_keys() {
    let mut fixture = CommandTestWorld::default();
    for id in ["demo:first", "demo:second", "other:hidden"] {
        fixture.setup(&format!("data merge storage {id} {{present:1}}"));
    }
    assert_eq!(
        texts(&completions_for(
            "data get storage demo:f",
            CommandPermission::Owner,
            &fixture
        )),
        ["demo:first"]
    );
}

#[test]
fn data_get_scale_formats_half_up_like_java() {
    let mut fixture = CommandTestWorld::default();
    fixture.setup("data merge storage demo:s {n:1}");
    // 0.125 is exact in binary64: Rust's `{:.2}` rounds half-to-even ("0.12"),
    // whereas Java's `String.format(Locale.ROOT, "%.2f", 0.125)` rounds half-up ("0.13").
    let run = ran("data get storage demo:s n 0.125", &mut fixture);
    assert_eq!(keys(&run), ["commands.data.storage.get"]);
    assert_eq!(arguments(&run, 0)[2], text("0.13"));
}
