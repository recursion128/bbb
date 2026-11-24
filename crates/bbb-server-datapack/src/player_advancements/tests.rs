use std::collections::BTreeMap;

use crate::advancement::{Advancement, AdvancementCriterion, AdvancementElement, AdvancementSet};
use bbb_protocol::{JavaString, NbtSummaryValue};
use bbb_registry_codec::{parse_registry_json, RegistryJsonValue};
use serde_json::{json, Value};

use super::*;

fn raw(value: Value) -> RegistryJsonValue {
    parse_registry_json(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn parse_document(value: &Value) -> Result<Java26PlayerAdvancements, PlayerAdvancementsParseError> {
    Java26PlayerAdvancements::parse_bytes(&serde_json::to_vec(value).unwrap())
}

fn nbt_field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn nbt(value: Value) -> NbtSummaryValue {
    bbb_protocol::json_value_to_nbt_summary(&value).unwrap()
}

/// Millis for `2026-08-31 02:30:45 +0000`, the instant the oracle documents in
/// this module were captured against.
const OBTAINED_MILLIS: i64 = 1_788_143_445_000;

fn element(id: &str, parent: Option<&str>, criteria: &[&str], body: Value) -> AdvancementElement {
    let criteria: Vec<(&str, &str)> = criteria
        .iter()
        .map(|name| (*name, "minecraft:impossible"))
        .collect();
    element_with_triggers(id, parent, &criteria, body)
}

fn element_with_triggers(
    id: &str,
    parent: Option<&str>,
    criteria: &[(&str, &str)],
    body: Value,
) -> AdvancementElement {
    AdvancementElement {
        id: id.to_string(),
        source_pack: "vanilla".to_string(),
        advancement: Advancement {
            rewards: Default::default(),
            parent: parent.map(str::to_string),
            criteria: criteria
                .iter()
                .map(|(name, trigger)| AdvancementCriterion {
                    name: (*name).into(),
                    trigger: (*trigger).to_string(),
                    conditions: RegistryJsonValue::Null,
                    block_contact: None,
                })
                .collect(),
            requirements: body.get("requirements").cloned().map(raw),
            display: body.get("display").cloned().map(raw),
            sends_telemetry_event: body
                .get("sends_telemetry_event")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
    }
}

fn shown() -> Value {
    json!({"display": {"icon": {"id": "minecraft:stone"}, "title": "t", "description": "d"}})
}

fn hidden() -> Value {
    json!({"display": {"icon": {"id": "minecraft:stone"}, "title": "t", "description": "d", "hidden": true}})
}

fn loaded(elements: Vec<AdvancementElement>) -> AdvancementSet {
    AdvancementSet {
        advancements: elements,
        warnings: Vec::new(),
    }
}

/// root -> child -> grandchild -> great_grandchild, all displayed, plus a
/// second root with no display at all.
fn sample_tree() -> AdvancementTree {
    AdvancementTree::from_loaded(&loaded(vec![
        element("minecraft:story/root", None, &["a"], shown()),
        element(
            "minecraft:story/child",
            Some("minecraft:story/root"),
            &["b"],
            shown(),
        ),
        element(
            "minecraft:story/grandchild",
            Some("minecraft:story/child"),
            &["c"],
            shown(),
        ),
        element(
            "minecraft:story/great_grandchild",
            Some("minecraft:story/grandchild"),
            &["d"],
            shown(),
        ),
        element("minecraft:recipes/root", None, &["e"], json!({})),
    ]))
}

fn index(tree: &AdvancementTree, id: &str) -> usize {
    tree.index_of(id).expect("the fixture declares this id")
}

fn added_ids(update: &AdvancementsUpdate) -> Vec<&str> {
    update
        .added
        .iter()
        .map(|summary| summary.id.as_str())
        .collect()
}

#[test]
fn requirements_default_to_all_of_the_criteria_and_validate_against_them() {
    let tree = AdvancementTree::from_loaded(&loaded(vec![element(
        "minecraft:story/root",
        None,
        &["a", "b"],
        shown(),
    )]));
    let requirements = tree
        .node(index(&tree, "minecraft:story/root"))
        .requirements();
    assert_eq!(
        requirements.groups(),
        &[vec![JavaString::from("a")], vec![JavaString::from("b")]]
    );
    // `test` (`AdvancementRequirements.java:40-52`): every group needs a name.
    assert!(!requirements.test(|name| name == "a"));
    assert!(requirements.test(|_| true));
    assert_eq!(requirements.count(|name| name == "b"), 1);
    // An empty grid is never satisfied (`:41-43`).
    assert!(!AdvancementRequirements::default().test(|_| true));
}

#[test]
fn a_declared_requirement_grid_replaces_the_default() {
    let tree = AdvancementTree::from_loaded(&loaded(vec![element(
        "minecraft:story/root",
        None,
        &["a", "b"],
        json!({"requirements": [["a", "b"]]}),
    )]));
    let requirements = tree
        .node(index(&tree, "minecraft:story/root"))
        .requirements();
    assert_eq!(requirements.size(), 1);
    assert!(requirements.test(|name| name == "b"));
}

#[test]
fn the_tree_resolves_parents_children_and_roots() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let child = index(&tree, "minecraft:story/child");
    let grandchild = index(&tree, "minecraft:story/grandchild");
    assert!(tree.node(root).is_root());
    assert_eq!(tree.node(child).parent(), Some(root));
    assert_eq!(tree.node(root).children(), [child]);
    assert_eq!(tree.node(grandchild).root(), root);
    assert_eq!(tree.roots().len(), 2);
    // `DisplayInfo.CODEC`'s defaults (`DisplayInfo.java:21-23`).
    let display = tree
        .node(root)
        .display()
        .expect("the fixture displays this");
    assert!(display.show_toast && display.announce_to_chat && !display.hidden);
    assert!(tree
        .node(index(&tree, "minecraft:recipes/root"))
        .display()
        .is_none());
}

#[test]
fn a_fresh_session_publishes_nothing() {
    let tree = sample_tree();
    let mut store = PlayerAdvancements::created(&tree);
    // Nothing marks a root for update, so the first flush has nothing to send
    // even though it is the first packet (`PlayerAdvancements.java:266-268`).
    assert_eq!(store.flush_dirty(&tree, true), None);
}

#[test]
fn a_completed_advancement_reveals_itself_and_two_generations_below_it() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    assert_eq!(
        store.award(&tree, root, &JavaString::from("a"), OBTAINED_MILLIS),
        AwardOutcome {
            granted: true,
            completed: true,
        }
    );
    let update = store.flush_dirty(&tree, true).expect("the root moved");
    assert!(update.reset);
    assert!(update.show_advancements);
    // `AdvancementVisibilityEvaluator.VISIBILITY_DEPTH` is 2 (`:12`, `:25-38`),
    // so the SHOW at the root reaches its child and grandchild and stops.
    // `evaluateVisibility` reports a node after its children (`:51-57`).
    assert_eq!(
        added_ids(&update),
        [
            "minecraft:story/grandchild",
            "minecraft:story/child",
            "minecraft:story/root",
        ]
    );
    assert!(update.removed.is_empty());
    let progressed: Vec<&str> = update.progress.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        progressed,
        [
            "minecraft:story/root",
            "minecraft:story/child",
            "minecraft:story/grandchild",
        ]
    );
    let (_, root_progress) = &update.progress[0];
    assert!(root_progress.is_done());
    assert_eq!(
        root_progress
            .criterion(&JavaString::from("a"))
            .and_then(CriterionProgress::obtained_epoch_millis),
        Some(OBTAINED_MILLIS)
    );
    // A second flush with nothing dirty sends nothing, and the reset flag is
    // spent (`:271`).
    assert_eq!(store.flush_dirty(&tree, true), None);
}

#[test]
fn a_hidden_unfinished_advancement_hides_the_generations_below_it() {
    let tree = AdvancementTree::from_loaded(&loaded(vec![
        element("minecraft:story/root", None, &["a"], shown()),
        element(
            "minecraft:story/child",
            Some("minecraft:story/root"),
            &["b"],
            hidden(),
        ),
        element(
            "minecraft:story/grandchild",
            Some("minecraft:story/child"),
            &["c"],
            shown(),
        ),
    ]));
    let mut store = PlayerAdvancements::created(&tree);
    store.award(
        &tree,
        index(&tree, "minecraft:story/root"),
        &JavaString::from("a"),
        OBTAINED_MILLIS,
    );
    let update = store.flush_dirty(&tree, true).expect("the root moved");
    assert_eq!(added_ids(&update), ["minecraft:story/root"]);
}

/// An advancement with no display at all is HIDE
/// (`AdvancementVisibilityEvaluator.java:15-17`), which is what stops a
/// completed ancestor's SHOW from reaching the generations under it -- the one
/// position the rule differs from the NO_CHANGE a plain displayed node returns
/// (`:25-38`).
#[test]
fn a_display_less_advancement_hides_the_generations_below_it() {
    let tree = AdvancementTree::from_loaded(&loaded(vec![
        element("minecraft:story/root", None, &["a"], shown()),
        element(
            "minecraft:story/child",
            Some("minecraft:story/root"),
            &["b"],
            json!({}),
        ),
        element(
            "minecraft:story/grandchild",
            Some("minecraft:story/child"),
            &["c"],
            shown(),
        ),
    ]));
    let mut store = PlayerAdvancements::created(&tree);
    store.award(
        &tree,
        index(&tree, "minecraft:story/root"),
        &JavaString::from("a"),
        OBTAINED_MILLIS,
    );
    let update = store.flush_dirty(&tree, true).expect("the root moved");
    assert_eq!(added_ids(&update), ["minecraft:story/root"]);

    // The rule is consulted only for a node no descendant has finished
    // (`:55`), so completing the grandchild reveals it and the display-less
    // node between them.
    assert!(
        store
            .award(
                &tree,
                index(&tree, "minecraft:story/grandchild"),
                &JavaString::from("c"),
                OBTAINED_MILLIS
            )
            .completed
    );
    let update = store
        .flush_dirty(&tree, true)
        .expect("the grandchild moved");
    let mut ids = added_ids(&update);
    ids.sort_unstable();
    assert_eq!(ids, ["minecraft:story/child", "minecraft:story/grandchild"]);
    assert!(store.is_visible(index(&tree, "minecraft:story/child")));
}

#[test]
fn revoking_the_last_criterion_takes_the_tree_back_out_of_sight() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    store.award(&tree, root, &JavaString::from("a"), OBTAINED_MILLIS);
    store.flush_dirty(&tree, true);
    assert!(store.revoke(&tree, root, &JavaString::from("a")));
    assert!(!store.revoke(&tree, root, &JavaString::from("a")));
    let update = store.flush_dirty(&tree, true).expect("the root moved back");
    assert!(!update.reset);
    assert!(update.added.is_empty());
    assert_eq!(
        update.removed,
        [
            "minecraft:story/grandchild",
            "minecraft:story/child",
            "minecraft:story/root",
        ]
    );
    // The revoked advancement is no longer visible, so its progress is not
    // published (`:259-263`).
    assert!(update.progress.is_empty());
}

#[test]
fn awarding_a_criterion_twice_moves_nothing() {
    let tree = sample_tree();
    let child = index(&tree, "minecraft:story/child");
    let mut store = PlayerAdvancements::created(&tree);
    store.award(&tree, child, &JavaString::from("b"), OBTAINED_MILLIS);
    assert_eq!(
        store.award(
            &tree,
            child,
            &JavaString::from("b"),
            OBTAINED_MILLIS + 1_000
        ),
        AwardOutcome::default()
    );
    assert_eq!(
        store
            .progress(child)
            .and_then(|progress| progress.criterion(&JavaString::from("b")))
            .and_then(CriterionProgress::obtained_epoch_millis),
        Some(OBTAINED_MILLIS)
    );
}

#[test]
fn only_a_displayed_root_can_be_the_selected_tab() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    assert_eq!(
        store.set_selected_tab(&tree, Some(root)),
        Some(Some("minecraft:story/root".to_string()))
    );
    // Selecting it again changed nothing, so nothing is sent (`:282`).
    assert_eq!(store.set_selected_tab(&tree, Some(root)), None);
    // A non-root clears the selection rather than being ignored (`:276-280`).
    assert_eq!(
        store.set_selected_tab(&tree, Some(index(&tree, "minecraft:story/child"))),
        Some(None)
    );
    assert_eq!(store.selected_tab(), None);
    // A root without a display is not a tab either.
    assert_eq!(
        store.set_selected_tab(&tree, Some(index(&tree, "minecraft:recipes/root"))),
        None
    );
}

#[test]
fn a_loaded_document_restores_progress_and_republishes_it() {
    let tree = sample_tree();
    let data = parse_document(&json!({
        "story/root": {"criteria": {"a": "2026-08-31 02:30:45 +0000"}, "done": true},
        "minecraft:story/gone": {"criteria": {"x": "2026-08-31 02:30:45 +0000"}, "done": true},
        "DataVersion": 4786,
    }))
    .expect("the document is well formed");
    let mut store = PlayerAdvancements::load(&tree, &data);
    let root = index(&tree, "minecraft:story/root");
    assert!(store.progress(root).expect("restored").is_done());
    let update = store
        .flush_dirty(&tree, true)
        .expect("the load marked the root");
    assert!(update.reset);
    assert!(added_ids(&update).contains(&"minecraft:story/root"));
    // Only advancements with obtained criteria go back out (`:157-165`).
    assert_eq!(
        store.to_data(&tree).progress.keys().collect::<Vec<_>>(),
        ["minecraft:story/root"]
    );
}

#[test]
fn undecodable_entries_and_criteria_are_dropped() {
    let data = parse_document(&json!({
        "Not An Id": {"done": true},
        "story/root": 7,
        "story/mine_stone": {"criteria": {"bad": "yesterday", "get_stone": "2026-08-31 02:30:45 +0000"}},
        "DataVersion": 4786,
    }))
    .expect("the document loads");
    assert_eq!(
        data.progress.keys().collect::<Vec<_>>(),
        ["minecraft:story/mine_stone"]
    );
    assert_eq!(
        data.progress["minecraft:story/mine_stone"]
            .criteria()
            .keys()
            .collect::<Vec<_>>(),
        ["get_stone"]
    );
}

#[test]
fn the_reader_normalizes_bare_paths_and_offsets() {
    let data = parse_document(&json!({
        "story/mine_stone": {"criteria": {"get_stone": "2025-12-31 23:59:59 -0500"}, "done": false},
        "DataVersion": 4786,
    }))
    .expect("the document is well formed");
    let progress = &data.progress["minecraft:story/mine_stone"];
    // The same instant the official codec reads as 2026-01-01T04:59:59Z.
    assert_eq!(
        progress
            .criterion(&JavaString::from("get_stone"))
            .and_then(CriterionProgress::obtained_epoch_millis),
        Some(1_767_243_599_000)
    );
    // A loaded entry carries no requirement grid, so it is not done yet
    // (`AdvancementProgress.java:37-43`, `:45`).
    assert!(!progress.is_done());
}

#[test]
fn the_document_round_trips_through_the_file_bytes() {
    let tree = sample_tree();
    let mut store = PlayerAdvancements::created(&tree);
    store.award(
        &tree,
        index(&tree, "minecraft:story/root"),
        &JavaString::from("a"),
        OBTAINED_MILLIS,
    );
    let data = store.to_data(&tree);
    let bytes = data.to_file_bytes();
    let reread = Java26PlayerAdvancements::parse_bytes(&bytes)
        .expect("the document this crate writes is one it reads");
    assert_eq!(
        reread.progress.keys().collect::<Vec<_>>(),
        ["minecraft:story/root"]
    );
    assert_eq!(
        reread.progress["minecraft:story/root"]
            .criterion(&JavaString::from("a"))
            .and_then(CriterionProgress::obtained_epoch_millis),
        Some(OBTAINED_MILLIS)
    );
    // Rebuilt against the tree, the restored entry is done again.
    assert!(PlayerAdvancements::load(&tree, &reread)
        .progress(index(&tree, "minecraft:story/root"))
        .expect("restored")
        .is_done());
}

#[test]
fn the_obtained_time_survives_a_round_trip_through_its_own_format() {
    for millis in [0_i64, OBTAINED_MILLIS, 1_767_243_599_000, -86_400_000] {
        assert_eq!(
            file::parse_obtained_time(&file::format_obtained_time(millis)),
            Some(millis)
        );
    }
    // The format drops everything below a second (`AdvancementProgress.java:26`).
    assert_eq!(
        file::parse_obtained_time(&file::format_obtained_time(OBTAINED_MILLIS + 999)),
        Some(OBTAINED_MILLIS)
    );
    assert_eq!(
        file::format_obtained_time(OBTAINED_MILLIS),
        "2026-08-31 02:30:45 +0000"
    );
}

/// The tree `scripts/vanilla-runtime/PlayerAdvancementsCodecReport.java`
/// declares beside the same document: a two-name `allOf` root, a child either
/// of whose two names satisfies it, and a one-name grandchild.
fn codec_oracle_tree() -> AdvancementTree {
    AdvancementTree::from_loaded(&loaded(vec![
        element("minecraft:story/root", None, &["a", "b"], shown()),
        element(
            "minecraft:story/child",
            Some("minecraft:story/root"),
            &["c", "d"],
            json!({
                "display": {"icon": {"id": "minecraft:stone"}, "title": "t", "description": "d"},
                "requirements": [["c", "d"]],
            }),
        ),
        element(
            "minecraft:story/grandchild",
            Some("minecraft:story/child"),
            &["e"],
            shown(),
        ),
    ]))
}

/// The document a save holds before the load, with a bare path, a criterion
/// the tree no longer names and an entry that obtained nothing.
fn codec_oracle_document() -> Java26PlayerAdvancements {
    parse_document(&json!({
        "minecraft:story/root": {
            "criteria": {
                "a": "2026-08-31 02:30:45 +0000",
                "stale": "2026-08-31 02:30:40 +0000",
            },
            "done": true,
        },
        "story/child": {"criteria": {"c": "2026-08-31 02:31:00 +0000"}, "done": false},
        "minecraft:story/grandchild": {"criteria": {}, "done": false},
        "DataVersion": 4786,
    }))
    .expect("the official jar reads this document")
}

/// `load` (`PlayerAdvancements.java:113-128`) through `startProgress`
/// (`:297-300`) and back out of `asData` (`:157-165`), against the 26.1 server
/// jar run over the same document and the same requirement grids by
/// `scripts/vanilla-runtime/PlayerAdvancementsCodecReport.java`.
#[test]
fn the_load_and_save_round_trip_matches_the_official_jar() {
    let tree = codec_oracle_tree();
    let store = PlayerAdvancements::load(&tree, &codec_oracle_document());

    // `progress minecraft:story/root done=false hasProgress=true percent=0.5
    // completed=[a] remaining=[b] firstProgressDate=2026-08-31T02:30:45Z`:
    // `update` dropped `stale` and added the unobtained `b`
    // (`AdvancementProgress.java:55-64`), and `isDone` reads the live grid
    // rather than the file's flag (`:66-68`).
    let root = store
        .progress(index(&tree, "minecraft:story/root"))
        .expect("the document names the root");
    assert!(!root.is_done());
    assert!(root.has_progress());
    assert_eq!(root.percent(), 0.5);
    assert_eq!(root.count_completed_requirements(), 1);
    assert_eq!(root.completed_criteria(), ["a"]);
    assert_eq!(root.remaining_criteria(), ["b"]);
    assert_eq!(
        root.criterion(&JavaString::from("a"))
            .and_then(CriterionProgress::obtained_epoch_millis),
        Some(OBTAINED_MILLIS)
    );
    assert!(root.criterion(&JavaString::from("stale")).is_none());

    // `progress minecraft:story/child done=true hasProgress=true percent=1.0
    // completed=[c] remaining=[d]`: one satisfied name carries the whole group
    // (`AdvancementRequirements.java:40-52`).
    let child = store
        .progress(index(&tree, "minecraft:story/child"))
        .expect("the bare path resolved to the child");
    assert!(child.is_done());
    assert_eq!(child.percent(), 1.0);
    assert_eq!(child.completed_criteria(), ["c"]);
    assert_eq!(child.remaining_criteria(), ["d"]);

    // `progress minecraft:story/grandchild done=false hasProgress=false
    // percent=0.0 completed=[] remaining=[e] firstProgressDate=null`.
    let grandchild = store
        .progress(index(&tree, "minecraft:story/grandchild"))
        .expect("the document names the grandchild");
    assert!(!grandchild.is_done());
    assert!(!grandchild.has_progress());
    assert_eq!(grandchild.percent(), 0.0);
    assert!(grandchild.completed_criteria().is_empty());

    // The jar's `document`, byte for byte apart from the key order a JSON
    // object does not carry: `asData` keeps only what obtained something
    // (`PlayerAdvancements.java:160`), the criteria codec writes only obtained names
    // (`AdvancementProgress.java:32-35`), `done` is re-derived (`:40`) and
    // `wrapCodec` merges the current data version in (`DataFixTypes.java:64-66`).
    assert_eq!(
        serde_json::to_value(store.to_data(&tree).to_document()).unwrap(),
        json!({
            "minecraft:story/root": {
                "criteria": {"a": "2026-08-31 02:30:45 +0000"},
                "done": false,
            },
            "minecraft:story/child": {
                "criteria": {"c": "2026-08-31 02:31:00 +0000"},
                "done": true,
            },
            "DataVersion": 4786,
        })
    );

    // The written document loads back to the same store.
    let rewritten = Java26PlayerAdvancements::parse_bytes(&store.to_data(&tree).to_file_bytes())
        .expect("the document this crate writes is one it reads");
    assert_eq!(
        PlayerAdvancements::load(&tree, &rewritten).to_data(&tree),
        store.to_data(&tree)
    );
}

/// Every case the 26.1 jar's `TreeNodePosition.run` was run over, as
/// `(case, shape, sibling order, laid-out location)`: the shape is
/// `(id, parent, displayed)` in insertion order, and each location is the
/// raw `f32` bits of the `DisplayInfo` x and y the jar left behind.
#[allow(clippy::type_complexity)]
const TREE_NODE_POSITION_ORACLE_26_1: &[(
    &str,
    &[(&str, &str, bool)],
    &[(&str, &[&str])],
    &[(&str, u32, u32)],
)] = &[
    (
        "lone_root",
        &[("root", "", true)],
        &[("root", &[])],
        &[("root", 0, 0)],
    ),
    (
        "mixed",
        &[
            ("root", "", true),
            ("a", "root", true),
            ("a1", "a", true),
            ("a1a", "a1", true),
            ("a1b", "a1", true),
            ("a2", "a", true),
            ("b", "root", false),
            ("b1", "b", true),
            ("b2", "b", false),
            ("b2a", "b2", true),
            ("c", "root", true),
            ("c1", "c", true),
            ("c1a", "c1", true),
            ("c1aa", "c1a", true),
            ("c2", "c", true),
            ("d", "root", true),
            ("d1", "d", true),
            ("d1a", "d1", true),
            ("d1b", "d1", true),
            ("d1c", "d1", true),
            ("e", "root", true),
        ],
        &[
            ("root", &["a", "b", "c", "d", "e"]),
            ("a", &["a1", "a2"]),
            ("a1", &["a1a", "a1b"]),
            ("a1a", &[]),
            ("a1b", &[]),
            ("a2", &[]),
            ("b", &["b1", "b2"]),
            ("b1", &[]),
            ("b2", &["b2a"]),
            ("b2a", &[]),
            ("c", &["c1", "c2"]),
            ("c1", &["c1a"]),
            ("c1a", &["c1aa"]),
            ("c1aa", &[]),
            ("c2", &[]),
            ("d", &["d1"]),
            ("d1", &["d1a", "d1b", "d1c"]),
            ("d1a", &[]),
            ("d1b", &[]),
            ("d1c", &[]),
            ("e", &[]),
        ],
        &[
            ("root", 0, 1081081856),
            ("a", 1065353216, 1065353216),
            ("a1", 1073741824, 1056964608),
            ("a1a", 1077936128, 0),
            ("a1b", 1077936128, 1065353216),
            ("a2", 1073741824, 1069547520),
            ("b1", 1065353216, 1073741824),
            ("b2a", 1065353216, 1077936128),
            ("c", 1065353216, 1082130432),
            ("c1", 1073741824, 1080033280),
            ("c1a", 1077936128, 1080033280),
            ("c1aa", 1082130432, 1080033280),
            ("c2", 1073741824, 1083179008),
            ("d", 1065353216, 1085276160),
            ("d1", 1073741824, 1085276160),
            ("d1a", 1077936128, 1083179008),
            ("d1b", 1077936128, 1085276160),
            ("d1c", 1077936128, 1087373312),
            ("e", 1065353216, 1087373312),
        ],
    ),
    (
        "wide_then_deep",
        &[
            ("root", "", true),
            ("a", "root", true),
            ("a1", "a", true),
            ("a2", "a", true),
            ("a3", "a", true),
            ("a4", "a", true),
            ("b", "root", true),
            ("b1", "b", true),
            ("b1a", "b1", true),
            ("b1aa", "b1a", true),
            ("b2", "b", true),
        ],
        &[
            ("root", &["a", "b"]),
            ("a", &["a1", "a2", "a3", "a4"]),
            ("a1", &[]),
            ("a2", &[]),
            ("a3", &[]),
            ("a4", &[]),
            ("b", &["b1", "b2"]),
            ("b1", &["b1a"]),
            ("b1a", &["b1aa"]),
            ("b1aa", &[]),
            ("b2", &[]),
        ],
        &[
            ("root", 0, 1077936128),
            ("a", 1065353216, 1069547520),
            ("a1", 1073741824, 0),
            ("a2", 1073741824, 1065353216),
            ("a3", 1073741824, 1073741824),
            ("a4", 1073741824, 1077936128),
            ("b", 1065353216, 1083179008),
            ("b1", 1073741824, 1082130432),
            ("b1a", 1077936128, 1082130432),
            ("b1aa", 1082130432, 1082130432),
            ("b2", 1073741824, 1084227584),
        ],
    ),
    (
        "deep_then_wide",
        &[
            ("root", "", true),
            ("a", "root", true),
            ("a1", "a", true),
            ("a1a", "a1", true),
            ("a1aa", "a1a", true),
            ("a1aaa", "a1aa", true),
            ("b", "root", true),
            ("b1", "b", true),
            ("b1a", "b1", true),
            ("b1b", "b1", true),
            ("b1c", "b1", true),
            ("b1d", "b1", true),
            ("c", "root", true),
            ("c1", "c", true),
            ("c1a", "c1", true),
            ("c1b", "c1", true),
        ],
        &[
            ("root", &["a", "b", "c"]),
            ("a", &["a1"]),
            ("a1", &["a1a"]),
            ("a1a", &["a1aa"]),
            ("a1aa", &["a1aaa"]),
            ("a1aaa", &[]),
            ("b", &["b1"]),
            ("b1", &["b1a", "b1b", "b1c", "b1d"]),
            ("b1a", &[]),
            ("b1b", &[]),
            ("b1c", &[]),
            ("b1d", &[]),
            ("c", &["c1"]),
            ("c1", &["c1a", "c1b"]),
            ("c1a", &[]),
            ("c1b", &[]),
        ],
        &[
            ("root", 0, 1076887552),
            ("a", 1065353216, 0),
            ("a1", 1073741824, 0),
            ("a1a", 1077936128, 0),
            ("a1aa", 1082130432, 0),
            ("a1aaa", 1084227584, 0),
            ("b", 1065353216, 1075838976),
            ("b1", 1073741824, 1075838976),
            ("b1a", 1077936128, 1065353216),
            ("b1b", 1077936128, 1073741824),
            ("b1c", 1077936128, 1077936128),
            ("b1d", 1077936128, 1082130432),
            ("c", 1065353216, 1085276160),
            ("c1", 1073741824, 1085276160),
            ("c1a", 1077936128, 1084227584),
            ("c1b", 1077936128, 1086324736),
        ],
    ),
    (
        "nested_offsets",
        &[
            ("root", "", true),
            ("a", "root", true),
            ("a1", "a", true),
            ("a1a", "a1", true),
            ("a1b", "a1", true),
            ("a1c", "a1", true),
            ("a2", "a", true),
            ("a2a", "a2", true),
            ("a2b", "a2", true),
            ("b", "root", true),
            ("b1", "b", true),
            ("b1a", "b1", true),
            ("b1b", "b1", true),
            ("b2", "b", true),
            ("b2a", "b2", true),
            ("b2b", "b2", true),
            ("b2c", "b2", true),
            ("c", "root", true),
            ("c1", "c", true),
        ],
        &[
            ("root", &["a", "b", "c"]),
            ("a", &["a1", "a2"]),
            ("a1", &["a1a", "a1b", "a1c"]),
            ("a1a", &[]),
            ("a1b", &[]),
            ("a1c", &[]),
            ("a2", &["a2a", "a2b"]),
            ("a2a", &[]),
            ("a2b", &[]),
            ("b", &["b1", "b2"]),
            ("b1", &["b1a", "b1b"]),
            ("b1a", &[]),
            ("b1b", &[]),
            ("b2", &["b2a", "b2b", "b2c"]),
            ("b2a", &[]),
            ("b2b", &[]),
            ("b2c", &[]),
            ("c", &["c1"]),
            ("c1", &[]),
        ],
        &[
            ("root", 0, 1085538304),
            ("a", 1065353216, 1074790400),
            ("a1", 1073741824, 1065353216),
            ("a1a", 1077936128, 0),
            ("a1b", 1077936128, 1065353216),
            ("a1c", 1077936128, 1073741824),
            ("a2", 1073741824, 1080033280),
            ("a2a", 1077936128, 1077936128),
            ("a2b", 1077936128, 1082130432),
            ("b", 1065353216, 1087897600),
            ("b1", 1073741824, 1085276160),
            ("b1a", 1077936128, 1084227584),
            ("b1b", 1077936128, 1086324736),
            ("b2", 1073741824, 1090519040),
            ("b2a", 1077936128, 1088421888),
            ("b2b", 1077936128, 1090519040),
            ("b2c", 1077936128, 1091567616),
            ("c", 1065353216, 1091567616),
            ("c1", 1073741824, 1091567616),
        ],
    ),
    (
        "third_walk",
        &[
            ("n0", "", true),
            ("n1", "n0", true),
            ("n2", "n0", true),
            ("n3", "n2", true),
            ("n4", "n0", true),
            ("n5", "n0", true),
            ("n6", "n4", true),
            ("n7", "n3", false),
            ("n8", "n4", true),
            ("n9", "n5", true),
            ("n10", "n2", true),
            ("n11", "n10", true),
            ("n12", "n7", true),
            ("n13", "n5", true),
            ("n14", "n2", true),
            ("n15", "n2", true),
        ],
        &[
            ("n0", &["n1", "n2", "n4", "n5"]),
            ("n1", &[]),
            ("n2", &["n3", "n10", "n14", "n15"]),
            ("n3", &["n7"]),
            ("n4", &["n6", "n8"]),
            ("n5", &["n9", "n13"]),
            ("n6", &[]),
            ("n7", &["n12"]),
            ("n8", &[]),
            ("n9", &[]),
            ("n10", &["n11"]),
            ("n11", &[]),
            ("n12", &[]),
            ("n13", &[]),
            ("n14", &[]),
            ("n15", &[]),
        ],
        &[
            ("n0", 0, 1080033280),
            ("n1", 1065353216, 1056964608),
            ("n2", 1065353216, 1069547520),
            ("n3", 1073741824, 0),
            ("n4", 1065353216, 1083179008),
            ("n5", 1065353216, 1087373312),
            ("n6", 1073741824, 1082130432),
            ("n8", 1073741824, 1084227584),
            ("n9", 1073741824, 1086324736),
            ("n10", 1073741824, 1065353216),
            ("n11", 1077936128, 1065353216),
            ("n12", 1077936128, 0),
            ("n13", 1073741824, 1088421888),
            ("n14", 1073741824, 1073741824),
            ("n15", 1073741824, 1077936128),
        ],
    ),
    (
        "hidden_chain",
        &[
            ("root", "", true),
            ("a", "root", false),
            ("a1", "a", false),
            ("a1a", "a1", true),
            ("a1b", "a1", true),
            ("b", "root", true),
            ("b1", "b", false),
            ("b1a", "b1", true),
            ("c", "root", false),
            ("c1", "c", false),
            ("c1a", "c1", false),
        ],
        &[
            ("root", &["a", "b", "c"]),
            ("a", &["a1"]),
            ("a1", &["a1a", "a1b"]),
            ("a1a", &[]),
            ("a1b", &[]),
            ("b", &["b1"]),
            ("b1", &["b1a"]),
            ("b1a", &[]),
            ("c", &["c1"]),
            ("c1", &["c1a"]),
            ("c1a", &[]),
        ],
        &[
            ("root", 0, 1065353216),
            ("a1a", 1065353216, 0),
            ("a1b", 1065353216, 1065353216),
            ("b", 1065353216, 1073741824),
            ("b1a", 1073741824, 1073741824),
        ],
    ),
];

/// The tree one oracle case describes, in the insertion order the jar was fed.
fn oracle_tree(shape: &[(&str, &str, bool)]) -> AdvancementTree {
    AdvancementTree::from_loaded(&loaded(
        shape
            .iter()
            .map(|(id, parent, displayed)| {
                element(
                    id,
                    (!parent.is_empty()).then_some(*parent),
                    &["a"],
                    if *displayed { shown() } else { json!({}) },
                )
            })
            .collect(),
    ))
}

#[test]
fn every_displayed_root_is_laid_out_where_the_official_jar_lays_it_out() {
    for (case, shape, siblings, expected) in TREE_NODE_POSITION_ORACLE_26_1 {
        let tree = oracle_tree(shape);
        // The jar orders siblings by identity hash, so each case only speaks
        // for this tree while the order it was captured under still holds.
        let order: Vec<(&str, Vec<&str>)> = shape
            .iter()
            .map(|(id, _, _)| {
                let children = tree
                    .node(index(&tree, id))
                    .children()
                    .iter()
                    .map(|child| tree.node(*child).id())
                    .collect();
                (*id, children)
            })
            .collect();
        let captured: Vec<(&str, Vec<&str>)> = siblings
            .iter()
            .map(|(id, children)| (*id, children.to_vec()))
            .collect();
        assert_eq!(order, captured, "{case}");

        let laid_out: Vec<(&str, u32, u32)> = shape
            .iter()
            .filter(|(_, _, displayed)| *displayed)
            .map(|(id, _, _)| {
                let display = tree
                    .node(index(&tree, id))
                    .display()
                    .expect("a displayed advancement keeps its display");
                (*id, display.x.to_bits(), display.y.to_bits())
            })
            .collect();
        assert_eq!(laid_out, expected.to_vec(), "{case}");
    }
}

#[test]
fn a_root_with_no_display_leaves_its_whole_tree_at_the_origin() {
    // `ServerAdvancementManager.apply` (`ServerAdvancementManager.java:44-47`)
    // runs the walk only for a root that has a display, and
    // `TreeNodePosition`'s constructor refuses an invisible one (`:28-30`), so
    // the displayed advancements below it keep the location `DisplayInfo`
    // starts on.
    let tree = oracle_tree(&[
        ("root", "", false),
        ("a", "root", true),
        ("a1", "a", true),
        ("b", "root", true),
    ]);
    for id in ["a", "a1", "b"] {
        let display = tree
            .node(index(&tree, id))
            .display()
            .expect("a displayed advancement keeps its display");
        assert_eq!((display.x, display.y), (0.0, 0.0), "{id}");
    }
}

#[test]
fn the_built_in_pack_lays_every_displayed_root_out_one_row_apart() {
    let startup = crate::startup::DatapackStartup::load(&crate::startup::DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: crate::registry_data_loader::DatapackLoadLimits::default(),
    })
    .expect("the embedded vanilla snapshot loads");
    let tree = AdvancementTree::from_loaded(&startup.advancements);
    assert_eq!((tree.len(), tree.roots().len()), (1617, 6));

    // A root is laid out on its own, so two advancements only have to keep
    // their distance from the ones sharing their root and their column.
    let mut columns: BTreeMap<(usize, u32), Vec<f32>> = BTreeMap::new();
    for node in 0..tree.len() {
        let Some(display) = tree.node(node).display() else {
            continue;
        };
        if tree.node(tree.node(node).root()).display().is_none() {
            assert_eq!(
                (display.x, display.y),
                (0.0, 0.0),
                "{}",
                tree.node(node).id()
            );
            continue;
        }
        // `secondWalk` (`TreeNodePosition.java:87`) writes the depth it was
        // reached at, which counts only the displayed advancements above it.
        let mut depth = 0;
        let mut above = tree.node(node).parent();
        while let Some(parent) = above {
            if tree.node(parent).display().is_some() {
                depth += 1;
            }
            above = tree.node(parent).parent();
        }
        assert_eq!(display.x, depth as f32, "{}", tree.node(node).id());
        columns
            .entry((tree.node(node).root(), display.x.to_bits()))
            .or_default()
            .push(display.y);
    }

    // The 125 the built-in pack displays, against the 1492 that exist only to
    // carry a `rewards.recipes` and have no display to position.
    assert_eq!(columns.values().map(Vec::len).sum::<usize>(), 125);
    let mut closest = f32::INFINITY;
    for column in columns.values_mut() {
        column.sort_by(f32::total_cmp);
        for pair in column.windows(2) {
            closest = closest.min(pair[1] - pair[0]);
        }
    }
    // What `apportion`'s `+ 1.0F` (`TreeNodePosition.java:156`) buys.
    assert_eq!(closest, 1.0);
}

#[test]
fn the_display_codec_reads_every_field_and_its_declared_default() {
    let tree = AdvancementTree::from_loaded(&loaded(vec![
        element(
            "minecraft:story/root",
            None,
            &["a"],
            json!({"display": {
                "icon": {"id": "minecraft:stone"},
                "title": "t",
                "description": "d",
            }}),
        ),
        element(
            "minecraft:story/child",
            Some("minecraft:story/root"),
            &["b"],
            json!({
                "display": {
                    "icon": {
                        "id": "decorated_pot",
                        "count": 3,
                        "components": {"minecraft:rarity": "uncommon"},
                    },
                    "title": {"translate": "advancements.story.root.title"},
                    "description": {"translate": "advancements.story.root.description"},
                    "background": "gui/advancements/backgrounds/stone",
                    "frame": "challenge",
                    "show_toast": false,
                    "announce_to_chat": false,
                    "hidden": true,
                },
                "sends_telemetry_event": true,
            }),
        ),
    ]));

    let root = index(&tree, "minecraft:story/root");
    let display = tree.node(root).display().expect("the fixture displays it");
    // `DisplayInfo.CODEC`'s four defaults (`DisplayInfo.java:20-23`), the
    // template's count (`ItemStackTemplate.java:24`) and its empty patch
    // (`:25`), and `Advancement.CODEC`'s telemetry default
    // (`Advancement.java:46`).
    assert_eq!(display.frame, AdvancementFrame::Task);
    assert!(display.show_toast && display.announce_to_chat && !display.hidden);
    assert_eq!(display.background, None);
    assert_eq!(display.icon, nbt(json!({"id":"minecraft:stone"})));
    assert!(!tree.node(root).sends_telemetry_event());

    let child = index(&tree, "minecraft:story/child");
    let display = tree.node(child).display().expect("the fixture displays it");
    assert_eq!(
        display.icon,
        nbt(
            json!({"id":"minecraft:decorated_pot", "count":3, "components":{"minecraft:rarity":"uncommon"}})
        )
    );
    assert_eq!(
        (
            display.title.component().clone(),
            display.description.component().clone(),
            display.background.clone(),
            display.frame,
        ),
        (
            nbt(json!({"translate": "advancements.story.root.title"})),
            nbt(json!({"translate": "advancements.story.root.description"})),
            Some("minecraft:gui/advancements/backgrounds/stone".to_string()),
            AdvancementFrame::Challenge,
        )
    );
    assert!(!display.show_toast && !display.announce_to_chat && display.hidden);
    assert!(tree.node(child).sends_telemetry_event());
}

#[test]
fn the_icon_reads_the_bare_item_id_its_alternative_accepts() {
    // `Codec.withAlternative(MAP_CODEC.codec(), Item.CODEC, ...)`
    // (`ItemStackTemplate.java:29`).
    let tree = AdvancementTree::from_loaded(&loaded(vec![element(
        "minecraft:story/root",
        None,
        &["a"],
        json!({"display": {"icon": "stone", "title": "t", "description": "d"}}),
    )]));
    assert_eq!(
        tree.node(index(&tree, "minecraft:story/root"))
            .display()
            .expect("the fixture displays it")
            .icon,
        nbt(json!({"id":"minecraft:stone"}))
    );
}

#[test]
fn the_added_set_carries_the_whole_advancement_the_packet_sends() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    store.award(&tree, root, &JavaString::from("a"), OBTAINED_MILLIS);
    let update = store.flush_dirty(&tree, true).expect("the root moved");
    let child = update
        .added
        .iter()
        .find(|summary| summary.id == "minecraft:story/child")
        .expect("the child came into sight with its root");
    // `Advancement.write` (`Advancement.java:83-88`): the parent, the display,
    // the requirement grid and the telemetry flag, with the criteria left off.
    assert_eq!(child.parent.as_deref(), Some("minecraft:story/root"));
    assert_eq!(child.requirements.groups(), &[vec![JavaString::from("b")]]);
    assert!(!child.sends_telemetry_event);
    assert_eq!(
        child.display.as_ref().map(|display| display.icon.clone()),
        Some(nbt(json!({"id":"minecraft:stone"})))
    );
    assert_eq!(
        tree.summary(index(&tree, "minecraft:recipes/root")).display,
        None
    );
}

#[test]
fn the_built_in_pack_keeps_every_display_the_official_pack_declares() {
    let startup = crate::startup::DatapackStartup::load(&crate::startup::DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: crate::registry_data_loader::DatapackLoadLimits::default(),
    })
    .expect("the embedded vanilla snapshot loads");
    let tree = AdvancementTree::from_loaded(&startup.advancements);
    assert_eq!(tree.len(), 1617);

    let mut frames: BTreeMap<&str, usize> = BTreeMap::new();
    let mut backgrounds: Vec<(&str, &str)> = Vec::new();
    let mut hidden = 0;
    let mut quiet = 0;
    let mut untoasted = 0;
    let mut patched: Vec<&str> = Vec::new();
    let mut telemetry = 0;
    for node in 0..tree.len() {
        if tree.node(node).sends_telemetry_event() {
            telemetry += 1;
        }
        let Some(display) = tree.node(node).display() else {
            continue;
        };
        *frames.entry(display.frame.serialized_name()).or_default() += 1;
        if let Some(background) = &display.background {
            backgrounds.push((tree.node(node).id(), background));
        }
        hidden += usize::from(display.hidden);
        quiet += usize::from(!display.announce_to_chat);
        untoasted += usize::from(!display.show_toast);
        if nbt_field(&display.icon, "components").is_some() {
            patched.push(tree.node(node).id());
        }
    }
    // The tree keeps a datapack order the ids do not follow, so these are
    // membership rather than sequence.
    backgrounds.sort_unstable();
    patched.sort_unstable();
    // The 125 the pack displays are exactly the 125 that report telemetry.
    assert_eq!(frames.values().sum::<usize>(), 125);
    assert_eq!(telemetry, 125);
    assert_eq!(
        frames.into_iter().collect::<Vec<(&str, usize)>>(),
        [("challenge", 25), ("goal", 10), ("task", 90)]
    );
    assert_eq!(
        backgrounds,
        [
            (
                "minecraft:adventure/root",
                "minecraft:gui/advancements/backgrounds/adventure"
            ),
            (
                "minecraft:end/root",
                "minecraft:gui/advancements/backgrounds/end"
            ),
            (
                "minecraft:husbandry/root",
                "minecraft:gui/advancements/backgrounds/husbandry"
            ),
            (
                "minecraft:nether/root",
                "minecraft:gui/advancements/backgrounds/nether"
            ),
            (
                "minecraft:story/root",
                "minecraft:gui/advancements/backgrounds/stone"
            ),
        ]
    );
    assert_eq!((hidden, quiet, untoasted), (9, 5, 5));
    assert_eq!(
        patched,
        [
            "minecraft:adventure/craft_decorated_pot_using_only_sherds",
            "minecraft:adventure/hero_of_the_village",
            "minecraft:adventure/voluntary_exile",
        ]
    );

    let root = tree.node(index(&tree, "minecraft:story/root"));
    let display = root.display().expect("the story root is displayed");
    assert_eq!(
        (
            nbt_field(&display.icon, "id").unwrap(),
            display.frame,
            display.show_toast,
            display.announce_to_chat,
        ),
        (
            &NbtSummaryValue::string("minecraft:grass_block"),
            AdvancementFrame::Task,
            false,
            false
        )
    );
    assert_eq!(
        display.title.component(),
        &nbt(json!({"translate": "advancements.story.root.title"}))
    );
}

fn listeners<'a>(store: &'a PlayerAdvancements, trigger: &str) -> Vec<(usize, &'a JavaString)> {
    store.trigger_listeners(trigger).collect()
}

#[test]
fn listeners_arm_only_criteria_still_owed() {
    let tree = AdvancementTree::from_loaded(&loaded(vec![element_with_triggers(
        "minecraft:story/root",
        None,
        &[("a", "minecraft:tick"), ("b", "minecraft:impossible")],
        json!({}),
    )]));
    let root = index(&tree, "minecraft:story/root");
    let store = PlayerAdvancements::created(&tree);
    assert_eq!(
        listeners(&store, "minecraft:tick"),
        [(root, &JavaString::from("a"))]
    );
    assert_eq!(
        listeners(&store, "minecraft:impossible"),
        [(root, &JavaString::from("b"))]
    );

    // `registerListeners` skips an obtained criterion (`:221-223`).
    let data = parse_document(&json!({
        "story/root": {"criteria": {"a": "2026-08-31 02:30:45 +0000"}, "done": false},
        "DataVersion": 4786,
    }))
    .expect("the document is well formed");
    let store = PlayerAdvancements::load(&tree, &data);
    assert_eq!(listeners(&store, "minecraft:tick"), []);
    assert_eq!(
        listeners(&store, "minecraft:impossible"),
        [(root, &JavaString::from("b"))]
    );
}

#[test]
fn award_and_revoke_move_the_listeners() {
    // One requirement group either criterion satisfies, so completing "a"
    // leaves "b" unobtained while the advancement is done.
    let tree = AdvancementTree::from_loaded(&loaded(vec![element_with_triggers(
        "minecraft:story/root",
        None,
        &[("a", "minecraft:tick"), ("b", "minecraft:impossible")],
        json!({"requirements": [["a", "b"]]}),
    )]));
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    assert_eq!(
        store.listened_triggers().collect::<Vec<_>>(),
        ["minecraft:impossible", "minecraft:tick"]
    );

    // `award` unregisters, and a done advancement takes every criterion's
    // listener with it (`:172`, `:237`).
    assert!(
        store
            .award(&tree, root, &JavaString::from("a"), OBTAINED_MILLIS)
            .completed
    );
    assert_eq!(store.listened_triggers().count(), 0);

    // `revoke` re-registers every criterion still owed (`:197`).
    assert!(store.revoke(&tree, root, &JavaString::from("a")));
    assert_eq!(
        listeners(&store, "minecraft:tick"),
        [(root, &JavaString::from("a"))]
    );
    assert_eq!(
        listeners(&store, "minecraft:impossible"),
        [(root, &JavaString::from("b"))]
    );
}

#[test]
fn an_empty_criteria_advancement_awards_automatically_and_stays_undone() {
    let tree = AdvancementTree::from_loaded(&loaded(vec![element(
        "minecraft:story/auto",
        None,
        &[],
        json!({}),
    )]));
    let auto = index(&tree, "minecraft:story/auto");
    let mut store = PlayerAdvancements::created(&tree);
    // `checkForAutomaticTriggers` started its progress, but an empty
    // requirement grid is never satisfied (`AdvancementRequirements.java:41-43`)
    // and there is no criterion the award could stamp.
    let progress = store
        .progress(auto)
        .expect("the automatic award started it");
    assert!(!progress.is_done());
    assert!(!progress.has_progress());
    // `getPercent` answers 0 for an empty criteria map rather than dividing by
    // the empty grid's size (`AdvancementProgress.java:124-126`).
    assert_eq!(progress.percent(), 0.0);
    assert_eq!(store.listened_triggers().count(), 0);
    assert!(store.to_data(&tree).progress.is_empty());
    assert_eq!(store.flush_dirty(&tree, true), None);
}

#[test]
fn reload_forgets_the_session_and_loads_the_new_document() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let data = parse_document(&json!({
        "story/root": {"criteria": {"a": "2026-08-31 02:30:45 +0000"}, "done": true},
        "DataVersion": 4786,
    }))
    .expect("the document is well formed");
    let mut store = PlayerAdvancements::created(&tree);
    store.award(&tree, root, &JavaString::from("a"), OBTAINED_MILLIS);
    store.set_selected_tab(&tree, Some(root));
    store.flush_dirty(&tree, true).expect("the root moved");

    store.reload(&tree, Some(&data));
    // `reload` clears the tab and the dirty sets and runs `load` again
    // (`:85-95`), so the next flush is a first packet carrying the document.
    assert_eq!(store.selected_tab(), None);
    assert_eq!(listeners(&store, "minecraft:impossible").len(), 4);
    let update = store.flush_dirty(&tree, true).expect("the load marked it");
    assert!(update.reset);
    assert!(added_ids(&update).contains(&"minecraft:story/root"));

    store.reload(&tree, None);
    assert!(store.to_data(&tree).progress.is_empty());
    assert_eq!(store.flush_dirty(&tree, true), None);
}

/// One displayed root whose two names share a single requirement group, so
/// either name alone completes it.
fn or_group_tree() -> AdvancementTree {
    AdvancementTree::from_loaded(&loaded(vec![element_with_triggers(
        "minecraft:story/root",
        None,
        &[("c", "minecraft:tick"), ("d", "minecraft:impossible")],
        json!({
            "display": {"icon": {"id": "minecraft:stone"}, "title": "t", "description": "d"},
            "requirements": [["c", "d"]],
        }),
    )]))
}

#[test]
fn a_group_is_satisfied_by_any_one_of_its_names() {
    let mut progress = AdvancementProgress::default();
    progress.update(AdvancementRequirements::new(vec![
        vec![JavaString::from("a"), JavaString::from("b")],
        vec![JavaString::from("c")],
    ]));
    assert!(!progress.is_done());
    assert_eq!(progress.count_completed_requirements(), 0);
    assert_eq!(progress.percent(), 0.0);

    assert!(progress.grant_progress(&JavaString::from("b"), OBTAINED_MILLIS));
    // `test` needs every group satisfied, `count` counts the satisfied ones
    // (`AdvancementRequirements.java:40-52`, `:54-64`), and `getPercent`
    // divides that count by the group count (`AdvancementProgress.java:123-131`).
    assert!(!progress.is_done());
    assert_eq!(progress.count_completed_requirements(), 1);
    assert_eq!(progress.percent(), 0.5);
    assert!(progress.has_progress());

    assert!(progress.grant_progress(&JavaString::from("c"), OBTAINED_MILLIS + 1_000));
    assert!(progress.is_done());
    assert_eq!(progress.percent(), 1.0);

    assert!(progress.grant_progress(&JavaString::from("a"), OBTAINED_MILLIS + 2_000));
    assert_eq!(progress.count_completed_requirements(), 2);
    assert_eq!(progress.percent(), 1.0);

    // `revokeProgress` answers false for a name that obtained nothing and for
    // one the map has no entry for at all (`AdvancementProgress.java:90-98`).
    assert!(progress.revoke_progress(&JavaString::from("a")));
    assert!(!progress.revoke_progress(&JavaString::from("a")));
    assert!(!progress.revoke_progress(&JavaString::from("not_a_criterion")));
    assert!(progress.is_done());
}

#[test]
fn fitting_a_loaded_entry_to_a_grid_drops_what_it_no_longer_names() {
    let mut progress = AdvancementProgress::from_criteria(BTreeMap::from([
        (
            JavaString::from("a"),
            CriterionProgress::obtained(OBTAINED_MILLIS),
        ),
        (
            JavaString::from("stale"),
            CriterionProgress::obtained(OBTAINED_MILLIS - 5_000),
        ),
    ]));
    // A parsed entry starts on `AdvancementRequirements.EMPTY`
    // (`AdvancementProgress.java:45`), which nothing satisfies.
    assert!(!progress.is_done());

    progress.update(AdvancementRequirements::all_of(["a", "b"]));
    // `update` removes the names the grid does not hold and adds the ones it
    // gained (`:55-64`). Only membership is pinned: vanilla's criteria map is
    // a `HashMap` (`:52`), so its iteration order is its own.
    assert_eq!(progress.criteria().len(), 2);
    assert!(progress.criterion(&JavaString::from("stale")).is_none());
    assert!(progress
        .criterion(&JavaString::from("a"))
        .is_some_and(CriterionProgress::is_done));
    assert!(progress
        .criterion(&JavaString::from("b"))
        .is_some_and(|criterion| !criterion.is_done()));
    assert_eq!(
        progress
            .criterion(&JavaString::from("a"))
            .and_then(CriterionProgress::obtained_epoch_millis),
        Some(OBTAINED_MILLIS)
    );
    assert!(progress.has_progress());
    assert!(!progress.is_done());
}

#[test]
fn awarding_a_name_the_advancement_does_not_hold_moves_nothing() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    // `grantProgress` answers false when the criteria map has no such entry
    // (`AdvancementProgress.java:80-88`), so neither dirty set is touched and
    // the first packet still has nothing to carry
    // (`PlayerAdvancements.java:266-268`).
    assert_eq!(
        store.award(
            &tree,
            root,
            &JavaString::from("not_a_criterion"),
            OBTAINED_MILLIS
        ),
        AwardOutcome::default()
    );
    assert!(!store.is_visible(root));
    assert_eq!(store.flush_dirty(&tree, true), None);
    assert!(store.to_data(&tree).progress.is_empty());
}

#[test]
fn a_done_advancement_grants_its_remaining_names_without_completing_again() {
    let tree = or_group_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    assert_eq!(
        store.award(&tree, root, &JavaString::from("c"), OBTAINED_MILLIS),
        AwardOutcome {
            granted: true,
            completed: true,
        }
    );
    // The reward and announcement arm hangs on `!wasDone && progress.isDone()`
    // (`PlayerAdvancements.java:175`), so the second name of a satisfied group
    // is granted without completing anything.
    assert_eq!(
        store.award(&tree, root, &JavaString::from("d"), OBTAINED_MILLIS + 1_000),
        AwardOutcome {
            granted: true,
            completed: false,
        }
    );
}

#[test]
fn revoking_one_name_of_a_satisfied_group_leaves_it_done() {
    let tree = or_group_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    store.award(&tree, root, &JavaString::from("c"), OBTAINED_MILLIS);
    store.award(&tree, root, &JavaString::from("d"), OBTAINED_MILLIS + 1_000);
    store.flush_dirty(&tree, true).expect("the root moved");

    assert!(store.revoke(&tree, root, &JavaString::from("c")));
    assert!(store
        .progress(root)
        .is_some_and(AdvancementProgress::is_done));
    // `registerListeners` returns before arming anything while the
    // advancement is done (`PlayerAdvancements.java:216-226`).
    assert_eq!(store.listened_triggers().count(), 0);

    let update = store
        .flush_dirty(&tree, true)
        .expect("the revoke republished the progress");
    // The visibility mark needs the done flag to fall (`:202-204`), so the
    // visible set did not move and only the progress went back out.
    assert!(update.added.is_empty());
    assert!(update.removed.is_empty());
    assert_eq!(update.progress.len(), 1);
    assert!(!store.revoke(&tree, root, &JavaString::from("c")));
    assert!(!store.revoke(&tree, root, &JavaString::from("not_a_criterion")));
}

#[test]
fn a_done_descendant_reveals_every_ancestor_above_it() {
    let tree = sample_tree();
    let deepest = index(&tree, "minecraft:story/great_grandchild");
    let mut store = PlayerAdvancements::created(&tree);
    assert!(
        store
            .award(&tree, deepest, &JavaString::from("d"), OBTAINED_MILLIS)
            .completed
    );
    let update = store.flush_dirty(&tree, true).expect("the tree moved");
    // `isSelfOrDescendantDone` is OR-ed back up the whole chain with no depth
    // limit (`AdvancementVisibilityEvaluator.java:46-58`); the two-generation
    // cap applies only to the rule that reaches unfinished nodes (`:12`).
    // Vanilla's added set is a `HashSet` (`PlayerAdvancements.java:250`), so
    // membership is what is pinned here, not the order.
    let mut ids = added_ids(&update);
    ids.sort_unstable();
    assert_eq!(
        ids,
        [
            "minecraft:story/child",
            "minecraft:story/grandchild",
            "minecraft:story/great_grandchild",
            "minecraft:story/root",
        ]
    );
    assert!(store.is_visible(index(&tree, "minecraft:story/root")));
    // Every node the walk revealed is published, because `updateTreeVisibility`
    // starts a progress entry for each one first (`:303`, `:308-310`).
    assert_eq!(update.progress.len(), 4);
}

#[test]
fn a_root_with_no_display_is_visible_once_it_is_done() {
    let tree = sample_tree();
    let recipes = index(&tree, "minecraft:recipes/root");
    let mut store = PlayerAdvancements::created(&tree);
    assert!(!store.is_visible(recipes));
    assert!(
        store
            .award(&tree, recipes, &JavaString::from("e"), OBTAINED_MILLIS)
            .completed
    );
    let update = store.flush_dirty(&tree, true).expect("the root moved");
    // The rule for an advancement with no display is HIDE
    // (`AdvancementVisibilityEvaluator.java:15-17`), but a done node is visible
    // before that rule is ever consulted (`:55`), so the tab-less root reaches
    // the client with no display attached.
    assert_eq!(added_ids(&update), ["minecraft:recipes/root"]);
    assert!(store.is_visible(recipes));
    assert!(update.added[0].display.is_none());
    // It is still not a tab (`PlayerAdvancements.java:276-280`).
    assert_eq!(store.set_selected_tab(&tree, Some(recipes)), None);
}

#[test]
fn clearing_the_selected_tab_answers_the_null_id_once() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    // Nothing is selected, so clearing it moves nothing
    // (`PlayerAdvancements.java:282`).
    assert_eq!(store.set_selected_tab(&tree, None), None);
    assert_eq!(
        store.set_selected_tab(&tree, Some(root)),
        Some(Some("minecraft:story/root".to_string()))
    );
    // Null is a value the field moves to, and the packet carries it
    // (`:274-285`).
    assert_eq!(store.set_selected_tab(&tree, None), Some(None));
    assert_eq!(store.set_selected_tab(&tree, None), None);
    assert_eq!(store.selected_tab(), None);
    assert_eq!(
        store.set_selected_tab(&tree, Some(root)),
        Some(Some("minecraft:story/root".to_string()))
    );
    assert_eq!(store.selected_tab(), Some(root));
}

/// Builtin membership beside a synthetic dynamic-registry generation: each
/// banner pattern's holder id is its position here.
const FIXTURE_BANNER_PATTERNS: [&str; 7] = [
    "minecraft:rhombus",
    "minecraft:stripe_bottom",
    "minecraft:stripe_center",
    "minecraft:border",
    "minecraft:stripe_middle",
    "minecraft:half_horizontal",
    "minecraft:circle",
];

struct FixtureHolderResolver;

impl bbb_protocol::PersistentRegistryResolver for FixtureHolderResolver {
    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> bbb_protocol::PersistentRegistryMembership {
        bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1(
            registry_id,
            resource_id,
        )
    }

    fn tag_members(
        &self,
        registry_id: &str,
        tag_id: &str,
    ) -> bbb_protocol::PersistentRegistryTagMembers {
        bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1(
            registry_id,
            tag_id,
        )
    }
}

impl bbb_registry_codec::PersistentItemStackHolderResolver for FixtureHolderResolver {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        (registry_id == "minecraft:banner_pattern")
            .then(|| {
                FIXTURE_BANNER_PATTERNS
                    .iter()
                    .position(|candidate| *candidate == resource_id)
            })
            .flatten()
            .map(|position| position as i32)
    }
}

fn item_id(resource_id: &str) -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
        .expect("the generated registry carries the fixture item")
        .id
}

#[test]
fn the_flushed_update_projects_onto_the_wire_shape() {
    let tree = sample_tree();
    let root = index(&tree, "minecraft:story/root");
    let mut store = PlayerAdvancements::created(&tree);
    store.award(&tree, root, &JavaString::from("a"), OBTAINED_MILLIS);
    let update = store.flush_dirty(&tree, true).expect("the root moved");
    let wire = advancements_update_to_wire(&update, &FixtureHolderResolver)
        .expect("the fixture icons project");

    assert!(wire.reset);
    assert!(wire.show_advancements);
    assert!(wire.removed.is_empty());
    assert_eq!(
        wire.added
            .iter()
            .map(|added| added.id.as_str())
            .collect::<Vec<_>>(),
        added_ids(&update)
    );
    let summary = wire
        .added
        .iter()
        .find(|added| added.id == "minecraft:story/root")
        .expect("the root is added");
    assert_eq!(summary.parent, None);
    assert_eq!(summary.requirements, [["a"]]);
    assert!(!summary.sends_telemetry_event);
    let display = summary.display.as_ref().expect("the root is displayed");
    assert_eq!(display.title.text(), "t");
    assert_eq!(
        *display.title.component(),
        bbb_protocol::NbtSummaryValue::string("t")
    );
    assert_eq!(display.icon.item_id, item_id("minecraft:stone"));
    assert_eq!(display.icon.count, 1);
    assert_eq!(
        display.icon.component_patch,
        bbb_protocol::packets::DataComponentPatchSummary::default()
    );
    assert_eq!(
        display.frame_type,
        bbb_protocol::packets::AdvancementFrameType::Task
    );
    assert!(display.show_toast && !display.hidden);
    assert_eq!(
        (display.background.as_deref(), display.x, display.y),
        (None, 0.0, 0.0)
    );
    let child = wire
        .added
        .iter()
        .find(|added| added.id == "minecraft:story/child")
        .expect("the child is added");
    assert_eq!(child.parent.as_deref(), Some("minecraft:story/root"));

    let progressed = wire
        .progress
        .iter()
        .find(|progress| progress.id == "minecraft:story/root")
        .expect("the root progressed");
    assert_eq!(
        progressed.criteria,
        [bbb_protocol::packets::AdvancementCriterionProgressSummary {
            name: JavaString::from("a"),
            obtained_epoch_millis: Some(OBTAINED_MILLIS),
        }]
    );
    let owed = wire
        .progress
        .iter()
        .find(|progress| progress.id == "minecraft:story/child")
        .expect("the newly visible child progressed");
    assert_eq!(
        owed.criteria,
        [bbb_protocol::packets::AdvancementCriterionProgressSummary {
            name: JavaString::from("b"),
            obtained_epoch_millis: None,
        }]
    );
}

#[test]
fn the_built_in_icons_project_their_component_patches() {
    let startup = crate::startup::DatapackStartup::load(&crate::startup::DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: crate::registry_data_loader::DatapackLoadLimits::default(),
    })
    .expect("the embedded vanilla snapshot loads");
    let tree = AdvancementTree::from_loaded(&startup.advancements);
    let update = AdvancementsUpdate {
        reset: true,
        added: [
            "minecraft:story/root",
            "minecraft:adventure/hero_of_the_village",
        ]
        .iter()
        .map(|id| tree.summary(index(&tree, id)))
        .collect(),
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: true,
    };
    let wire = advancements_update_to_wire(&update, &FixtureHolderResolver)
        .expect("the built-in icons project");

    let story = wire.added[0].display.as_ref().expect("story is displayed");
    assert_eq!(
        *story.title.component(),
        bbb_protocol::NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
            name: "translate".into(),
            value: bbb_protocol::NbtSummaryValue::string("advancements.story.root.title"),
        }])
    );
    assert_eq!(
        story.background.as_deref(),
        Some("minecraft:gui/advancements/backgrounds/stone")
    );

    // The ominous banner: eight layers, a translated item name, uncommon
    // rarity and a tooltip that hides the pattern list.
    let hero = wire.added[1].display.as_ref().expect("hero is displayed");
    assert_eq!(hero.icon.item_id, item_id("minecraft:white_banner"));
    let patch = &hero.icon.component_patch;
    let layers: Vec<(Option<i32>, i32)> = patch
        .banner_pattern_layers
        .iter()
        .map(|layer| (layer.registry_id, layer.color_id))
        .collect();
    // Colors are `DyeColor` ids: cyan 9, light_gray 8, gray 7, black 15.
    assert_eq!(
        layers,
        [
            (Some(0), 9),
            (Some(1), 8),
            (Some(2), 7),
            (Some(3), 8),
            (Some(4), 15),
            (Some(5), 8),
            (Some(6), 8),
            (Some(3), 15),
        ]
    );
    assert_eq!(
        patch.rarity,
        Some(bbb_protocol::item_component_value_types::ItemRaritySummary::Uncommon)
    );
    assert_eq!(
        patch.tooltip_hidden_component_type_ids,
        [bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID]
    );
    assert!(patch.item_name_nbt.is_some());

    // The third patched icon in the pack is the decorated pot, whose
    // `minecraft:pot_decorations` the projection now owns. `ordered` writes
    // the four sides back with an absent one as brick
    // (`PotDecorations.java:50-52`), so the pack's two sherds come through in
    // their declared positions.
    let pot = AdvancementsUpdate {
        reset: true,
        added: vec![tree.summary(index(
            &tree,
            "minecraft:adventure/craft_decorated_pot_using_only_sherds",
        ))],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: true,
    };
    let wire = advancements_update_to_wire(&pot, &FixtureHolderResolver)
        .expect("the pot decorations projection is owned");
    let patch = &wire.added[0]
        .display
        .as_ref()
        .expect("the pot advancement has a display")
        .icon
        .component_patch;
    assert_eq!(
        patch.pot_decorations_item_ids,
        [
            item_id("minecraft:brick"),
            item_id("minecraft:heart_pottery_sherd"),
            item_id("minecraft:brick"),
            item_id("minecraft:explorer_pottery_sherd"),
        ]
    );
}

#[test]
fn raw_criterion_identity_survives_awards_save_reload_and_typed_sync() {
    let source = parse_registry_json(
        br#"{
        "criteria": {
            "\ud800": {"trigger":"minecraft:tick"},
            "\ud801": {"trigger":"minecraft:tick"},
            "\ufffd": {"trigger":"minecraft:tick"},
            "\ud83d\ude00": {"trigger":"minecraft:tick"}
        },
        "display":{"icon":"minecraft:stone","title":{"text":"\ud800"},"description":"d"}
    }"#,
    )
    .unwrap();
    let advancement = crate::advancement::parse_advancement(&source).unwrap();
    let tree = AdvancementTree::from_loaded(&loaded(vec![AdvancementElement {
        id: "minecraft:raw".to_string(),
        source_pack: "fixture".to_string(),
        advancement,
    }]));
    assert!(tree.warnings().is_empty(), "{:?}", tree.warnings());
    let root = index(&tree, "minecraft:raw");
    let names: Vec<JavaString> = [
        vec![0xd800],
        vec![0xd801],
        vec![0xfffd],
        vec![0xd83d, 0xde00],
    ]
    .into_iter()
    .map(JavaString::from)
    .collect();
    let mut store = PlayerAdvancements::created(&tree);
    for (offset, name) in names.iter().enumerate() {
        assert!(store.is_listening("minecraft:tick", root, name));
        assert!(
            store
                .award(&tree, root, name, OBTAINED_MILLIS + offset as i64 * 1000)
                .granted
        );
        assert!(!store.is_listening("minecraft:tick", root, name));
    }
    assert!(store.progress(root).unwrap().is_done());
    let saved = store.to_data(&tree);
    let saved_json = String::from_utf8(saved.to_file_bytes()).unwrap();
    let deep_ignored = format!("{}0{}", "[".repeat(80), "]".repeat(80));
    let ignored = format!("[{deep_ignored},{}null]", "null,".repeat(4096));
    let input = format!("{{\"ignored\":{ignored},{}", &saved_json[1..]);
    let reread = Java26PlayerAdvancements::parse_bytes(input.as_bytes()).unwrap();
    let mut restored = PlayerAdvancements::load(&tree, &reread);
    assert_eq!(restored.to_data(&tree), saved);
    for name in &names {
        assert!(restored
            .progress(root)
            .unwrap()
            .criterion(name)
            .unwrap()
            .is_done());
    }
    let wire = advancements_update_to_wire(
        &restored.flush_dirty(&tree, true).unwrap(),
        &FixtureHolderResolver,
    )
    .unwrap();
    let typed_names: std::collections::BTreeSet<_> = wire.progress[0]
        .criteria
        .iter()
        .map(|criterion| criterion.name.clone())
        .collect();
    assert_eq!(typed_names, names.into_iter().collect());
    let title = &wire.added[0].display.as_ref().unwrap().title;
    assert_eq!(
        title.component(),
        &NbtSummaryValue::string_from_utf16(vec![0xd800])
    );
}
