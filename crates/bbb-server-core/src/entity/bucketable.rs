//! Bucketable mob capture and the item components retained by its bucket.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
};
use bbb_protocol::item_types::{vanilla_item_type_for_resource_id, VANILLA_ITEM_WATER_BUCKET_ID};
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::NbtSummaryValue;

use super::ai::MemoryModuleType;
use super::fish::FishLeafState;
use super::store::PendingEntitySound;
use super::{Entity, EntityData, EntityStore, RemovalReason};

/// `Bucketable.bucketMobPickup` (`Bucketable.java:72-91`).
pub(super) fn pickup(entity: &Entity, hand: &ItemStackSummary) -> Option<ItemStackSummary> {
    if hand.item_id != Some(VANILLA_ITEM_WATER_BUCKET_ID)
        || hand.count <= 0
        || !entity.is_alive_with_health()
    {
        return None;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let (item, _) = bucket_kind(data.entity_resource_id())?;
    let mut bucket = ItemStackSummary {
        item_id: Some(vanilla_item_type_for_resource_id(item)?.id),
        count: 1,
        component_patch: Default::default(),
    };
    let patch = &mut bucket.component_patch;
    let mut entries = Vec::new();
    // `Bucketable.saveDefaultDataToBucketTag` (`Bucketable.java:33-58`).
    for (name, enabled) in [
        ("NoAI", data.is_no_ai()),
        ("Silent", entity.is_silent()),
        ("NoGravity", entity.no_gravity()),
        ("Glowing", entity.has_glowing_tag()),
        ("Invulnerable", entity.invulnerable()),
    ] {
        if enabled {
            entries.push(entry(name, NbtSummaryValue::Byte(1)));
        }
    }
    entries.push(entry(
        "Health",
        NbtSummaryValue::Float(entity.living()?.health().to_bits()),
    ));
    if let Some(name) = entity.custom_name() {
        patch.custom_name_nbt = Some(name.component().clone());
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID);
    }
    match data.fish_leaf() {
        // `Salmon.saveToBucketTag` (`Salmon.java:101-104`).
        Some(FishLeafState::Salmon(size)) => {
            patch
                .builtin_enum_component_ids
                .insert(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, size.id());
            patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID);
        }
        // `TropicalFish.saveToBucketTag` (`TropicalFish.java:201-206`).
        Some(FishLeafState::Tropical(variant)) => {
            let packed = variant.packed();
            patch.tropical_fish_pattern_id = Some(packed & 0xffff);
            patch.tropical_fish_base_color_id = Some((packed >> 16) & 0xff);
            patch.tropical_fish_pattern_color_id = Some((packed >> 24) & 0xff);
            patch.added_type_ids.extend([
                VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
                VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
                VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
            ]);
        }
        _ => {}
    }
    // `Axolotl.saveToBucketTag` (`Axolotl.java:429-440`).
    if data.entity_resource_id() == "minecraft:axolotl" {
        patch.builtin_enum_component_ids.insert(
            VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
            data.component_simple_variant()?,
        );
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID);
        let age = data.ageable_retained()?;
        entries.extend([
            entry("Age", NbtSummaryValue::Int(age.age())),
            entry(
                "AgeLocked",
                NbtSummaryValue::Byte(i8::from(age.age_locked())),
            ),
        ]);
        if let Some(memory) =
            super::spawner_brain_memory(entity, MemoryModuleType::HasHuntingCooldown)
        {
            entries.push(entry(
                "HuntingCooldown",
                NbtSummaryValue::Long(memory.time_to_live.unwrap_or(i64::MAX)),
            ));
        }
    }
    // `Tadpole.saveToBucketTag` (`Tadpole.java:183-189`).
    if let Some(age) = data.tadpole_retained() {
        entries.extend([
            entry("Age", NbtSummaryValue::Int(age.age())),
            entry(
                "AgeLocked",
                NbtSummaryValue::Byte(i8::from(age.age_locked())),
            ),
        ]);
    }
    patch.codec_backed_nbt.insert(
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        NbtSummaryValue::Compound(entries),
    );
    patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID);
    Some(bucket)
}

pub(crate) fn finish_capture(store: &mut EntityStore, entity_id: i32) {
    let Some(entity) = store.get(entity_id) else {
        return;
    };
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some((_, sound)) = bucket_kind(data.entity_resource_id()) else {
        return;
    };
    let sound = (!entity.is_silent()).then(|| PendingEntitySound {
        position: entity.position(),
        resource_id: sound.into(),
        source: super::living::living_sound_source(entity),
        volume: 1.0,
        pitch: 1.0,
    });
    let _ = store.remove(entity_id, RemovalReason::Discarded);
    if let Some(sound) = sound {
        store.play_entity_sound(sound);
    }
}

fn bucket_kind(resource_id: &str) -> Option<(&'static str, &'static str)> {
    Some(match resource_id {
        "minecraft:cod" => ("minecraft:cod_bucket", "minecraft:item.bucket.fill_fish"),
        "minecraft:salmon" => ("minecraft:salmon_bucket", "minecraft:item.bucket.fill_fish"),
        "minecraft:pufferfish" => (
            "minecraft:pufferfish_bucket",
            "minecraft:item.bucket.fill_fish",
        ),
        "minecraft:tropical_fish" => (
            "minecraft:tropical_fish_bucket",
            "minecraft:item.bucket.fill_fish",
        ),
        "minecraft:tadpole" => (
            "minecraft:tadpole_bucket",
            "minecraft:item.bucket.fill_tadpole",
        ),
        "minecraft:axolotl" => (
            "minecraft:axolotl_bucket",
            "minecraft:item.bucket.fill_axolotl",
        ),
        _ => return None,
    })
}
