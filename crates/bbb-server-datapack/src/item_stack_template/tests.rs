use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_SHULKER_BOX_ID, VANILLA_ITEM_STICK_ID,
};
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_registry_codec::{decode_direct_registry_json, RegistryElementSource};
use serde_json::{json, Value};

use super::{remove_component, set_component, ItemStackTemplate};
use crate::recipe::test_support::builtin_lookup;

fn template(value: Value) -> ItemStackTemplate {
    let bytes = serde_json::to_vec(&value).unwrap();
    let decoded = decode_direct_registry_json(&bytes).unwrap();
    ItemStackTemplate::from_source(
        decoded.root(),
        RegistryElementSource::ServerDatapackJson,
        &builtin_lookup(),
    )
    .unwrap()
}

fn keyed(type_ids: &[i32]) -> DataComponentPatchSummary {
    DataComponentPatchSummary {
        added_type_ids: type_ids.to_vec(),
        ..DataComponentPatchSummary::default()
    }
}

#[test]
fn create_folds_registered_defaults_and_removals_without_one() {
    let shulker = template(json!({
        "id": "minecraft:shulker_box",
        "components": { "minecraft:container": [] }
    }))
    .create();
    assert_eq!(shulker.item_id, Some(VANILLA_ITEM_SHULKER_BOX_ID));
    assert!(shulker.component_patch.added_type_ids.is_empty());

    let stew = |remainder: &str| {
        template(json!({
            "id": "minecraft:mushroom_stew",
            "components": { "minecraft:use_remainder": { "id": remainder } }
        }))
        .create()
        .component_patch
    };
    assert!(stew("minecraft:bowl").added_type_ids.is_empty());
    let bucket = stew("minecraft:bucket");
    assert_eq!(
        bucket.added_type_ids,
        [VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID]
    );
    assert_eq!(
        bucket.use_remainder.as_ref().unwrap().item_id,
        VANILLA_ITEM_BUCKET_ID
    );

    let crossbow = template(json!({
        "id": "minecraft:crossbow",
        "components": {
            "!minecraft:charged_projectiles": {},
            "!minecraft:container": {},
            "minecraft:bundle_contents": []
        }
    }))
    .create()
    .component_patch;
    assert_eq!(
        crossbow.removed_type_ids,
        [VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID]
    );
    assert_eq!(
        crossbow.added_type_ids,
        [VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID]
    );
}

#[test]
fn apply_rebuilds_the_carried_patch_from_its_keyed_values() {
    let bundle = template(json!({
        "id": "minecraft:bundle",
        "components": { "minecraft:max_damage": 4 }
    }));
    let mut carried = keyed(&[
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    ]);
    carried.damage = Some(2);
    carried.max_damage = Some(9);
    carried.removed_type_ids = vec![VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID];
    carried.repair_cost = Some(7);
    let result = bundle.apply(1, &carried).unwrap().component_patch;
    assert_eq!(
        result.added_type_ids,
        [
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
        ]
    );
    assert!(result.removed_type_ids.is_empty());
    assert_eq!(result.damage, Some(2));
    assert_eq!(result.max_damage, Some(4));
    assert_eq!(result.repair_cost, None);

    let mut patch = result.clone();
    let defaults = bundle.prototype_defaults();
    remove_component(
        &mut patch,
        defaults,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    )
    .unwrap();
    remove_component(&mut patch, defaults, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID).unwrap();
    assert_eq!(
        patch.removed_type_ids,
        [VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID]
    );
    assert_eq!(
        patch.added_type_ids,
        [VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID]
    );
    assert_eq!(patch.damage, None);
    let empty_contents = keyed(&[VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID]);
    set_component(
        &mut patch,
        defaults,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        &empty_contents,
    )
    .unwrap();
    assert!(patch.removed_type_ids.is_empty());
    assert_eq!(
        patch.added_type_ids,
        [VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID]
    );
}

/// `new ItemStack(item, count, patch)` keeps a non-positive count
/// (`ItemStack.java:256-264`), and validation reads such a stack as empty.
#[test]
fn a_non_positive_count_builds_a_live_empty_stack() {
    let template = template(json!({
        "id": "minecraft:stick",
        "components": { "minecraft:max_damage": 5 }
    }));
    let created = template.apply(-3, &Default::default()).unwrap();
    assert_eq!(created.item_id, Some(VANILLA_ITEM_STICK_ID));
    assert_eq!(created.count, -3);
    assert_eq!(created.component_patch.max_damage, Some(5));
    assert_eq!(template.apply(0, &Default::default()).unwrap().count, 0);
    // A stick stacks to 64, so the same patch on a real stack is refused.
    assert_eq!(
        template.apply(2, &Default::default()).unwrap().item_id,
        None
    );
}

#[test]
fn registered_holder_defaults_resolve_through_the_supplied_lookup() {
    let disc = |song: &str| {
        template(json!({
            "id": "minecraft:music_disc_13",
            "components": { "minecraft:jukebox_playable": song }
        }))
        .create()
        .component_patch
    };
    assert!(disc("minecraft:13").added_type_ids.is_empty());
    assert_eq!(
        disc("minecraft:cat").added_type_ids,
        [VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID]
    );
}
