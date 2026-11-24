use super::support::{default_state, origin, place_feature, retained_from_json, RecordingLevel};
use super::UPDATE_CLIENTS;

pub(crate) fn simple_block_placed(block: &str) -> String {
    format!(
        r#"{{"feature":{{"type":"minecraft:simple_block","config":{{"to_place":{{"type":"minecraft:simple_state_provider","state":{{"Name":"{block}"}}}}}}}},"placement":[]}}"#
    )
}

fn random_selector_json(chance: &str) -> String {
    format!(
        r#"{{"type":"minecraft:random_selector","config":{{"features":[{{"feature":{stone},"chance":{chance}}}],"default":{dirt}}}}}"#,
        stone = simple_block_placed("minecraft:stone"),
        dirt = simple_block_placed("minecraft:dirt"),
    )
}

#[test]
fn random_selector_takes_the_first_passing_chance_or_the_default() {
    let mut level = RecordingLevel::admitting();
    let always = retained_from_json(random_selector_json("1.0").as_bytes());
    assert!(place_feature(&always, &mut level, origin()));
    assert_eq!(
        *level.sets.borrow(),
        vec![(origin(), default_state("minecraft:stone"), UPDATE_CLIENTS)]
    );

    let mut level = RecordingLevel::admitting();
    let never = retained_from_json(random_selector_json("0.0").as_bytes());
    assert!(place_feature(&never, &mut level, origin()));
    assert_eq!(
        *level.sets.borrow(),
        vec![(origin(), default_state("minecraft:dirt"), UPDATE_CLIENTS)]
    );
}

#[test]
fn simple_random_selector_indexes_the_holder_set_with_one_draw() {
    let json = format!(
        r#"{{"type":"minecraft:simple_random_selector","config":{{"features":[{stone},{dirt}]}}}}"#,
        stone = simple_block_placed("minecraft:stone"),
        dirt = simple_block_placed("minecraft:dirt"),
    );
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(
        &retained_from_json(json.as_bytes()),
        &mut level,
        origin()
    ));
    let index = bbb_server_worldgen::random::WorldgenRandom::with_seed(1).next_int(2);
    let picked = [
        default_state("minecraft:stone"),
        default_state("minecraft:dirt"),
    ][index as usize];
    assert_eq!(
        *level.sets.borrow(),
        vec![(origin(), picked, UPDATE_CLIENTS)]
    );

    // A one-element set collapses to the bare holder and needs no draw.
    let single = format!(
        r#"{{"type":"minecraft:simple_random_selector","config":{{"features":[{dirt}]}}}}"#,
        dirt = simple_block_placed("minecraft:dirt"),
    );
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(
        &retained_from_json(single.as_bytes()),
        &mut level,
        origin()
    ));
    assert_eq!(
        *level.sets.borrow(),
        vec![(origin(), default_state("minecraft:dirt"), UPDATE_CLIENTS)]
    );
}

#[test]
fn random_boolean_selector_follows_one_next_boolean_draw() {
    let json = format!(
        r#"{{"type":"minecraft:random_boolean_selector","config":{{"feature_true":{stone},"feature_false":{dirt}}}}}"#,
        stone = simple_block_placed("minecraft:stone"),
        dirt = simple_block_placed("minecraft:dirt"),
    );
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(
        &retained_from_json(json.as_bytes()),
        &mut level,
        origin()
    ));
    let mut random = bbb_server_worldgen::random::WorldgenRandom::with_seed(1);
    let picked = if random.next_boolean() {
        default_state("minecraft:stone")
    } else {
        default_state("minecraft:dirt")
    };
    assert_eq!(
        *level.sets.borrow(),
        vec![(origin(), picked, UPDATE_CLIENTS)]
    );
}
