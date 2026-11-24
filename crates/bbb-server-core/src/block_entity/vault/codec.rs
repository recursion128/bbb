use bbb_protocol::nbt_summary::codec_double_value as number;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::Result;
use uuid::Uuid;

use super::data::DEFAULT_LOOT_TABLE;
use super::{VaultConfig, VaultServerData, VaultSharedData};
use crate::block_entity::{
    codec_uuid, entry, unique_field, uuid_int_array, BlockEntityItemLoad, BlockEntityItemSave,
};
use bbb_server_types::item_stack::RuntimeItemStack;

// VaultConfig.java:26-36: the optional fields are lenient before range validation.
impl VaultConfig {
    pub(super) fn from_nbt(
        value: &NbtSummaryValue,
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<Option<Self>> {
        let NbtSummaryValue::Compound(fields) = value else {
            return Ok(None);
        };
        let config = Self {
            loot_table: unique_field(fields, "loot_table")
                .and_then(resource_id)
                .unwrap_or_else(|| DEFAULT_LOOT_TABLE.into()),
            activation_range: unique_field(fields, "activation_range")
                .and_then(number)
                .unwrap_or(4.0),
            deactivation_range: unique_field(fields, "deactivation_range")
                .and_then(number)
                .unwrap_or(4.5),
            key_item: optional_item(fields, "key_item", load)?,
            override_loot_table_to_display: unique_field(fields, "override_loot_table_to_display")
                .and_then(resource_id),
        };
        Ok((config
            .activation_range
            .partial_cmp(&config.deactivation_range)
            != Some(std::cmp::Ordering::Greater))
        .then_some(config))
    }
    pub(super) fn to_nbt(&self, save: &mut BlockEntityItemSave<'_>) -> Result<NbtSummaryValue> {
        let mut fields = Vec::new();
        if self.loot_table != DEFAULT_LOOT_TABLE {
            fields.push(entry(
                "loot_table",
                NbtSummaryValue::string(self.loot_table.clone()),
            ));
        }
        if self.activation_range != 4.0 {
            fields.push(entry(
                "activation_range",
                NbtSummaryValue::Double(self.activation_range.to_bits()),
            ));
        }
        if self.deactivation_range != 4.5 {
            fields.push(entry(
                "deactivation_range",
                NbtSummaryValue::Double(self.deactivation_range.to_bits()),
            ));
        }
        write_optional_item(&mut fields, "key_item", &self.key_item, save)?;
        if let Some(id) = &self.override_loot_table_to_display {
            fields.push(entry(
                "override_loot_table_to_display",
                NbtSummaryValue::string(id.clone()),
            ));
        }
        Ok(NbtSummaryValue::Compound(fields))
    }
}

// VaultSharedData.java:16-24.
impl VaultSharedData {
    pub(super) fn from_nbt(
        value: &NbtSummaryValue,
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<Option<Self>> {
        let NbtSummaryValue::Compound(fields) = value else {
            return Ok(None);
        };
        Ok(Some(Self {
            display_item: optional_item(fields, "display_item", load)?,
            connected_players: unique_field(fields, "connected_players")
                .and_then(uuid_set)
                .unwrap_or_default(),
            connected_particles_range: unique_field(fields, "connected_particles_range")
                .and_then(number)
                .unwrap_or(4.5),
            dirty: false,
        }))
    }
    pub(super) fn to_nbt(&self, save: &mut BlockEntityItemSave<'_>) -> Result<NbtSummaryValue> {
        let mut fields = Vec::new();
        write_optional_item(&mut fields, "display_item", &self.display_item, save)?;
        if !self.connected_players.is_empty() {
            fields.push(entry(
                "connected_players",
                uuid_set_nbt(&self.connected_players),
            ));
        }
        if self.connected_particles_range != 4.5 {
            fields.push(entry(
                "connected_particles_range",
                NbtSummaryValue::Double(self.connected_particles_range.to_bits()),
            ));
        }
        Ok(NbtSummaryValue::Compound(fields))
    }
}

// VaultServerData.java:20-27.
impl VaultServerData {
    pub(super) fn from_nbt(
        value: &NbtSummaryValue,
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<Option<Self>> {
        let NbtSummaryValue::Compound(fields) = value else {
            return Ok(None);
        };
        let mut items_to_eject = Vec::new();
        let mut invalid_items = false;
        if let Some(NbtSummaryValue::List(items)) = unique_field(fields, "items_to_eject") {
            for value in items {
                if let Some(item) = load.read(value)? {
                    items_to_eject.push(item);
                } else {
                    invalid_items = true;
                }
            }
        }
        if invalid_items {
            items_to_eject.clear();
        }
        Ok(Some(Self {
            rewarded_players: unique_field(fields, "rewarded_players")
                .and_then(uuid_set)
                .unwrap_or_default(),
            state_updating_resumes_at: unique_field(fields, "state_updating_resumes_at")
                .and_then(long)
                .unwrap_or(0),
            items_to_eject,
            total_ejections_needed: unique_field(fields, "total_ejections_needed")
                .and_then(int)
                .unwrap_or(0),
            last_insert_fail_timestamp: 0,
            dirty: false,
        }))
    }
    pub(super) fn to_nbt(&self, save: &mut BlockEntityItemSave<'_>) -> Result<NbtSummaryValue> {
        let mut fields = Vec::new();
        if !self.rewarded_players.is_empty() {
            fields.push(entry(
                "rewarded_players",
                uuid_set_nbt(&self.rewarded_players),
            ));
        }
        if self.state_updating_resumes_at != 0 {
            fields.push(entry(
                "state_updating_resumes_at",
                NbtSummaryValue::Long(self.state_updating_resumes_at),
            ));
        }
        if !self.items_to_eject.is_empty() {
            let mut items = Vec::new();
            for item in &self.items_to_eject {
                match save.write(item)? {
                    Some(item) => items.push(item),
                    None => {
                        items.clear();
                        break;
                    }
                }
            }
            if !items.is_empty() {
                fields.push(entry("items_to_eject", NbtSummaryValue::List(items)));
            }
        }
        if self.total_ejections_needed != 0 {
            fields.push(entry(
                "total_ejections_needed",
                NbtSummaryValue::Int(self.total_ejections_needed),
            ));
        }
        Ok(NbtSummaryValue::Compound(fields))
    }
}

fn optional_item(
    fields: &[NbtSummaryEntry],
    name: &'static str,
    load: &mut BlockEntityItemLoad<'_>,
) -> Result<RuntimeItemStack> {
    match unique_field(fields, name) {
        Some(value) => Ok(load.read(value)?.unwrap_or_else(RuntimeItemStack::empty)),
        None => Ok(RuntimeItemStack::empty()),
    }
}
fn write_optional_item(
    fields: &mut Vec<NbtSummaryEntry>,
    name: &'static str,
    item: &RuntimeItemStack,
    save: &mut BlockEntityItemSave<'_>,
) -> Result<()> {
    if !item.is_empty() {
        if let Some(value) = save.write(item)? {
            fields.push(entry(name, value));
        }
    }
    Ok(())
}
fn resource_id(value: &NbtSummaryValue) -> Option<String> {
    let NbtSummaryValue::String(value) = value else {
        return None;
    };
    value
        .try_to_string()
        .ok()
        .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())
}
fn uuid_set(value: &NbtSummaryValue) -> Option<Vec<Uuid>> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    let mut result = Vec::new();
    for value in values {
        let uuid = codec_uuid(value)?;
        if !result.contains(&uuid) {
            result.push(uuid);
        }
    }
    Some(result)
}
fn uuid_set_nbt(values: &[Uuid]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .map(|uuid| NbtSummaryValue::IntArray(uuid_int_array(*uuid)))
            .collect(),
    )
}

fn long(value: &NbtSummaryValue) -> Option<i64> {
    match value {
        NbtSummaryValue::Long(value) => Some(*value),
        _ => number(value).map(|value| value as i64),
    }
}
fn int(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Long(value) => Some(*value as i32),
        _ => number(value).map(|value| value as i32),
    }
}
