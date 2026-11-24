//! `MushroomCow.Variant` (`MushroomCow.java:259-263`), the red-or-brown the
//! mooshroom keeps in `DATA_TYPE` (`:54`, `:208-214`) and saves as `"Type"`
//! (`:190-199`).
//!
//! The variant lives in the entity's saved `"Type"` field, which is what the
//! loot views already read (`mob_death_loot.rs`, `sheep_shear.rs`). Keeping it
//! there leaves one source for the save, the loot reads and the synched row.

use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_synched_data::VANILLA_MUSHROOM_COW_TYPE_DATA_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use bbb_protocol::NbtSummaryValue;

use crate::entity::living::SynchedValue;
use crate::entity::{Entity, EntityData};

/// `MushroomCow.Variant.RED` (`MushroomCow.java:260`), which is also
/// `Variant.DEFAULT` (`:263`).
pub(crate) const RED: i32 = 0;
/// `MushroomCow.Variant.BROWN` (`MushroomCow.java:261`).
pub(crate) const BROWN: i32 = 1;

const FIELD: &str = "Type";

const fn name_of(variant: i32) -> &'static str {
    if variant == BROWN {
        "brown"
    } else {
        "red"
    }
}

fn id_of(name: &str) -> Option<i32> {
    match name {
        "red" => Some(RED),
        "brown" => Some(BROWN),
        _ => None,
    }
}

/// `MushroomCow.getVariant()` (`MushroomCow.java:212-214`), or nothing for a
/// body that is not a mooshroom or whose saved field this build cannot read.
pub(crate) fn variant_of(entity: &Entity) -> Option<i32> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    if data.entity_resource_id() != "minecraft:mooshroom" {
        return None;
    }
    data.opaque_fields()
        .iter()
        .find(|entry| entry.name == FIELD)
        .and_then(|entry| match &entry.value {
            NbtSummaryValue::String(value) => value.with_utf8(id_of).flatten(),
            _ => None,
        })
}

/// The other of the two, which is the swap `getOffspringVariant` makes on its
/// 1-in-1024 draw (`MushroomCow.java:251`) and `thunderHit` makes outright
/// (`:78`).
pub(crate) const fn other_variant(variant: i32) -> i32 {
    if variant == BROWN {
        RED
    } else {
        BROWN
    }
}

/// `MushroomCow.setVariant` (`MushroomCow.java:208-210`): the saved field and
/// the synched row move together.
pub(crate) fn set_variant(entity: &mut Entity, variant: i32) -> bool {
    let name = name_of(variant);
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    if data.entity_resource_id() != "minecraft:mooshroom" {
        return false;
    }
    if !data.replace_opaque_string(FIELD, name) {
        return false;
    }
    entity.mark_synched_dirty(VANILLA_MUSHROOM_COW_TYPE_DATA_ID);
    true
}

/// `MushroomCow.defineSynchedData` (`MushroomCow.java:87-89`), whose default is
/// `Variant.DEFAULT.id`.
pub(crate) fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    let Some(variant) = variant_of(entity) else {
        return Vec::new();
    };
    vec![SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_MUSHROOM_COW_TYPE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(variant),
        },
        is_default: variant == RED,
    }]
}

/// `MushroomCow.mobInteract` (`MushroomCow.java:92-165`).
pub(crate) fn mob_interact(
    store: &mut crate::entity::store::EntityStore,
    target_id: i32,
    hand: &mut bbb_protocol::packets::ItemStackSummary,
    infinite_materials: bool,
) -> Option<crate::entity::leash::LeashInteraction> {
    use crate::entity::leash::LeashInteraction;
    use bbb_protocol::item_types::{
        vanilla_suspicious_stew_ingredient, VANILLA_ITEM_BOWL_ID, VANILLA_ITEM_MUSHROOM_STEW_ID,
        VANILLA_ITEM_SUSPICIOUS_STEW_ID,
    };
    use bbb_protocol::packets::{ItemStackSummary, SoundSource, SuspiciousStewEffectSummary};
    let target = store.get_mut(target_id)?;
    if !target.is_alive_with_health() || hand.count <= 0 {
        return None;
    }
    let variant = variant_of(target)?;
    let position = target.position();
    let silent = target.is_silent();
    let EntityData::GeneralLiving(data) = &mut target.data else {
        return None;
    };
    if data.ageable_retained().is_some_and(|age| age.is_baby()) {
        return None;
    }
    let fields = data.opaque_fields_mut();
    let effects_index = fields.iter().position(|entry| entry.name == "stew_effects");
    let (outcome, sound, volume) = if hand.item_id == Some(VANILLA_ITEM_BOWL_ID) {
        let effects = effects_index.and_then(|index| {
            let NbtSummaryValue::List(entries) = &fields[index].value else {
                return None;
            };
            entries
                .iter()
                .map(|entry| {
                    let NbtSummaryValue::Compound(entry) = entry else {
                        return None;
                    };
                    let id = entry.iter().find(|field| field.name == "id")?;
                    let NbtSummaryValue::String(id) = &id.value else {
                        return None;
                    };
                    let id = id
                        .try_to_string()
                        .ok()
                        .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())?;
                    let effect_id =
                        bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(
                            &id,
                        )?
                        .id;
                    let duration = entry
                        .iter()
                        .find(|field| field.name == "duration")
                        .and_then(|field| bbb_protocol::nbt_summary::codec_int_value(&field.value))
                        .unwrap_or(160);
                    Some(SuspiciousStewEffectSummary {
                        effect_id,
                        duration,
                    })
                })
                .collect::<Option<Vec<_>>>()
        });
        let suspicious = effects.is_some();
        let mut item = ItemStackSummary {
            item_id: Some(if suspicious {
                VANILLA_ITEM_SUSPICIOUS_STEW_ID
            } else {
                VANILLA_ITEM_MUSHROOM_STEW_ID
            }),
            count: 1,
            component_patch: Default::default(),
        };
        if let Some(effects) = effects {
            let patch = &mut item.component_patch;
            patch.added_type_ids.push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID);
            patch.suspicious_stew_effects = effects;
            fields.retain(|entry| entry.name != "stew_effects");
        }
        (
            LeashInteraction::MooshroomStew { item },
            if suspicious {
                "minecraft:entity.mooshroom.suspicious_milk"
            } else {
                "minecraft:entity.mooshroom.milk"
            },
            1.0,
        )
    } else {
        if variant != BROWN {
            return None;
        }
        let ingredient = vanilla_suspicious_stew_ingredient(hand.item_id?)?;
        if effects_index.is_some() {
            return Some(LeashInteraction::MooshroomFed);
        }
        let effect =
            bbb_protocol::mob_effect_types::vanilla_mob_effect_type(ingredient.effect_type_id)?;
        fields.push(bbb_protocol::NbtSummaryEntry {
            name: "stew_effects".into(),
            value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                bbb_protocol::NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String(effect.resource_id.into()),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "duration".into(),
                    value: NbtSummaryValue::Int(ingredient.duration_ticks),
                },
            ])]),
        });
        if !infinite_materials {
            hand.count -= 1;
            if hand.count == 0 {
                *hand = ItemStackSummary::empty();
            }
        }
        (
            LeashInteraction::MooshroomFed,
            "minecraft:entity.mooshroom.eat",
            2.0,
        )
    };
    if !silent {
        store.play_entity_sound(crate::entity::store::PendingEntitySound {
            position,
            resource_id: sound.into(),
            source: SoundSource::Neutral,
            volume,
            pitch: 1.0,
        });
    }
    Some(outcome)
}

/// `MushroomCow.shear` and `readyForShearing` (`MushroomCow.java:172-187`).
pub(crate) fn shear(
    store: &mut crate::entity::EntityStore,
    target_id: i32,
    hand: &bbb_protocol::packets::ItemStackSummary,
    source: bbb_protocol::packets::SoundSource,
    loot: Option<&mut crate::entity::shearing_loot::ShearingLoot<'_>>,
) -> bool {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    use bbb_protocol::packets::{LevelParticles, ParticlePayload, Vec3d};
    if hand.item_id != Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID) || hand.count <= 0 {
        return false;
    }
    let Some(target) = store.get(target_id) else {
        return false;
    };
    let EntityData::GeneralLiving(data) = target.data() else {
        return false;
    };
    if variant_of(target).is_none() || !target.is_alive_with_health() || data.is_baby() {
        return false;
    }
    let Some(loot) = loot else {
        return false;
    };
    let position = target.position();
    let height = target.bounding_box().max_y - target.bounding_box().min_y;
    let snapshot =
        crate::entity::conversion::SingleMobConversionSnapshot::capture_with(target, false, false);
    let drops = crate::entity::shearing_loot::draw_loot(
        "minecraft:shearing/mooshroom",
        store,
        target,
        hand,
        loot,
    )
    .unwrap_or_default();
    let uuid = store.next_uuid();
    let Some(cow_id) = store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_COW_ID,
        position,
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap(),
        ),
        |cow| snapshot.apply(cow),
    ) else {
        return false;
    };
    crate::entity::conversion::transfer_single_relations(store, target_id, cow_id);
    store.play_entity_sound(crate::entity::store::PendingEntitySound {
        position,
        resource_id: "minecraft:entity.mooshroom.shear".into(),
        source,
        volume: 1.0,
        pitch: 1.0,
    });
    store.send_particles(LevelParticles {
        override_limiter: false,
        always_show: false,
        position: Vec3d {
            y: position.y + height * 0.5,
            ..position
        },
        offset: Vec3d::default(),
        max_speed: 0.0,
        count: 1,
        particle: ParticlePayload {
            particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
            raw_options: Vec::new(),
        },
    });
    for drop in drops {
        for _ in 0..drop.count() {
            crate::entity::spawn_dropped_item_entity(
                store,
                Vec3d {
                    y: position.y + height,
                    ..position
                },
                drop.with_count(1),
            );
        }
    }
    store.remove(target_id, crate::entity::RemovalReason::Discarded);
    true
}
