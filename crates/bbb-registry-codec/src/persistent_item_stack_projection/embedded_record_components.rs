//! `minecraft:lock` and `minecraft:lodestone_tracker`: persistent codecs that
//! are records over data the summary already keeps verbatim, with no registry
//! holder to name.

use bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID;
use bbb_protocol::packets::{BlockPos, DataComponentPatchSummary, LodestoneTargetSummary};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::registry_codec::RegistryElementValueRef;

use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::nbt_support::{normalize_resource_id_units, persistent_flag_nbt, ComponentReader};

/// `minecraft:lock` is `persistent(LockCode.CODEC)` (`DataComponents.java:323`)
/// with no `networkSynchronized`, so its network value already is its save value
/// `LockCode.CODEC` is
/// `ItemPredicate.CODEC.xmap`, which is exactly the
/// `ItemPredicate` compound `encode_persistent_item_predicate_summary` writes
///
/// The summary keeps that compound's own fields, so both directions go through
/// the wire codec's own pair and nothing is re-derived.
pub(super) fn persistent_lock_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
) -> Result<NbtSummaryValue> {
    let predicate = patch.lock_predicate.as_ref().ok_or(
        ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        },
    )?;
    bbb_protocol::packets::encode_persistent_item_predicate_summary(predicate)
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))
}

pub(super) fn project_lock(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let invalid = |error: bbb_protocol::codec::ProtocolError| {
        invalid_persistent_component(type_id, resource_id, error.to_string())
    };
    let predicate =
        bbb_protocol::packets::decode_persistent_item_predicate_summary(value).map_err(invalid)?;
    patch.lock_predicate = Some(predicate);
    Ok(())
}

/// `minecraft:lodestone_tracker` is `persistent(LodestoneTracker.CODEC)`
/// (`DataComponents.java:292-294`), two `optionalFieldOf` fields
/// `target`, a `GlobalPos`, and `tracked`,
/// defaulting to `true`.
///
/// `GlobalPos.CODEC` is `{dimension, pos}`, the
/// dimension a `ResourceKey.codec` identifier and the
/// position `Codec.INT_STREAM` over three ints, which
/// `NbtOps.createIntList` writes as an `IntArrayTag`.
pub(super) fn persistent_lodestone_tracker_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
) -> Result<NbtSummaryValue> {
    if patch.lodestone_target.is_none() && patch.lodestone_tracked.is_none() {
        return Err(ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        });
    }
    let mut fields = Vec::new();
    if let Some(target) = &patch.lodestone_target {
        let dimension = bbb_protocol::normalize_resource_location(target.dimension.as_str())
            .map_err(|error| {
                invalid_persistent_component(type_id, resource_id, error.to_string())
            })?;
        fields.push(NbtSummaryEntry {
            name: "target".into(),
            value: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "dimension".into(),
                    value: NbtSummaryValue::String(dimension.into()),
                },
                NbtSummaryEntry {
                    name: "pos".into(),
                    value: NbtSummaryValue::IntArray(vec![
                        target.pos.x,
                        target.pos.y,
                        target.pos.z,
                    ]),
                },
            ]),
        });
    }
    // `optionalFieldOf("tracked", true)` drops the field while it holds `true`.
    if patch.lodestone_tracked == Some(false) {
        fields.push(NbtSummaryEntry {
            name: "tracked".into(),
            value: persistent_flag_nbt(false),
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

pub fn read_lodestone_tracker(
    value: RegistryElementValueRef<'_>,
) -> Result<(Option<LodestoneTargetSummary>, bool)> {
    let c = ComponentReader::new(
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
        "minecraft:lodestone_tracker",
    );
    if !value.is_compound() {
        return Err(c.error("lodestone tracker is not a compound"));
    }
    let target = value
        .field("target")
        .map(|target| project_global_pos(c, target))
        .transpose()?;
    let tracked = value
        .field("tracked")
        .map(|value| c.flag(value, "tracked"))
        .transpose()?
        .unwrap_or(true);
    Ok((target, tracked))
}

fn project_global_pos(
    c: ComponentReader,
    value: RegistryElementValueRef<'_>,
) -> Result<LodestoneTargetSummary> {
    let dimension = value
        .field("dimension")
        .and_then(RegistryElementValueRef::java_string_ref)
        .ok_or_else(|| c.error("global position carries no dimension string"))?;
    let coordinates = value
        .field("pos")
        .and_then(RegistryElementValueRef::list_elements)
        .filter(|coordinates| coordinates.len() == 3)
        .ok_or_else(|| c.error("global position must carry three coordinates"))?;
    let mut pos = [0; 3];
    for (output, value) in pos.iter_mut().zip(coordinates) {
        *output = c.int(value, "global position coordinate")?;
    }
    Ok(LodestoneTargetSummary {
        dimension: normalize_resource_id_units(dimension.units())?,
        pos: BlockPos {
            x: pos[0],
            y: pos[1],
            z: pos[2],
        },
    })
}
