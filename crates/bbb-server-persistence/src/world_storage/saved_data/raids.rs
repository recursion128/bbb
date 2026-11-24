use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use crate::level_file::read_gzip_compound;
use crate::schema_support::{
    bool_value, compound_or_empty, field, float_value, int_array, int_value, list_or_empty,
    long_value, require_java_26_1_root, uuid_nbt, uuid_value,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{nbt_summary::entry, NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::{
    Java26LevelKey, NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths,
    JAVA_26_1_DATA_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Java26RaidStatus {
    Ongoing,
    Victory,
    Loss,
    Stopped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26Raid {
    pub started: bool,
    pub active: bool,
    pub ticks_active: i64,
    pub raid_omen_level: i32,
    pub groups_spawned: i32,
    pub cooldown_ticks: i32,
    pub post_raid_ticks: i32,
    pub total_health: f32,
    pub group_count: i32,
    pub status: Java26RaidStatus,
    pub center: BlockPos,
    pub heroes_of_the_village: BTreeSet<Uuid>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26Raids {
    pub raids: BTreeMap<i32, Java26Raid>,
    pub next_id: i32,
    pub tick: i32,
}

impl Default for Java26Raids {
    fn default() -> Self {
        Self {
            raids: BTreeMap::new(),
            next_id: 1,
            tick: 0,
        }
    }
}

pub fn load_java_26_1_raids(
    paths: &WorldSavePaths,
    dimension: &Java26LevelKey,
    limits: WorldSaveLimits,
) -> SaveResult<Java26Raids> {
    let path = paths
        .dimension_dir(dimension)
        .join("data/minecraft/raids.dat");
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Java26Raids::default())
        }
        Err(error) => {
            return Err(crate::atomic_file::io_error(
                "reading raid saved data metadata",
                &path,
                error,
            ))
        }
        Ok(_) => {}
    }
    let (document, _) = read_gzip_compound(&path, limits)?;
    parse_java_26_1_raids_document(&document)
}

// Raids.java:31-58,181-184: id and Raid.MAP_CODEC share each list row; a row
// that does not decode is dropped and a document without next_id or tick is
// a fresh Raids.
pub fn parse_java_26_1_raids_document(document: &NamedCompound) -> SaveResult<Java26Raids> {
    let data = compound_or_empty(require_java_26_1_root(document, "raids")?, "data");
    let (Some(next_id), Some(tick)) = (
        field(data, "next_id").and_then(int_value),
        field(data, "tick").and_then(int_value),
    ) else {
        return Ok(Java26Raids::default());
    };
    let raids = list_or_empty(data, "raids")
        .iter()
        .filter_map(|row| {
            let NbtSummaryValue::Compound(fields) = row else {
                return None;
            };
            Some((
                field(fields, "id").and_then(int_value)?,
                parse_raid(fields)?,
            ))
        })
        .collect();
    Ok(Java26Raids {
        raids,
        next_id,
        tick,
    })
}

// Raid.java:67-83,811-817.
fn parse_raid(fields: &[NbtSummaryEntry]) -> Option<Java26Raid> {
    let int = |name: &str| field(fields, name).and_then(int_value);
    let boolean = |name: &str| field(fields, name).and_then(bool_value);
    let status = match field(fields, "status")? {
        NbtSummaryValue::String(status) => match status.try_to_string().ok()?.as_str() {
            "ongoing" => Java26RaidStatus::Ongoing,
            "victory" => Java26RaidStatus::Victory,
            "loss" => Java26RaidStatus::Loss,
            "stopped" => Java26RaidStatus::Stopped,
            _ => return None,
        },
        _ => return None,
    };
    let [x, y, z] = field(fields, "center").and_then(int_array)?;
    let NbtSummaryValue::List(heroes) = field(fields, "heroes_of_the_village")? else {
        return None;
    };
    Some(Java26Raid {
        started: boolean("started")?,
        active: boolean("active")?,
        ticks_active: field(fields, "ticks_active").and_then(long_value)?,
        raid_omen_level: int("raid_omen_level")?,
        groups_spawned: int("groups_spawned")?,
        cooldown_ticks: int("cooldown_ticks")?,
        post_raid_ticks: int("post_raid_ticks")?,
        total_health: field(fields, "total_health").and_then(float_value)?,
        group_count: int("group_count")?,
        status,
        center: BlockPos { x, y, z },
        heroes_of_the_village: heroes.iter().filter_map(uuid_value).collect(),
    })
}

pub fn java_26_1_raids_document(data: &Java26Raids) -> NamedCompound {
    let raids = data
        .raids
        .iter()
        .map(|(id, raid)| {
            let status = match raid.status {
                Java26RaidStatus::Ongoing => "ongoing",
                Java26RaidStatus::Victory => "victory",
                Java26RaidStatus::Loss => "loss",
                Java26RaidStatus::Stopped => "stopped",
            };
            let health = if raid.total_health == 0.0 {
                0.0
            } else if raid.total_health.is_nan() {
                f32::NAN
            } else {
                raid.total_health
            };
            let heroes = raid
                .heroes_of_the_village
                .iter()
                .copied()
                .map(uuid_nbt)
                .collect();
            NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::Int(*id)),
                entry("started", NbtSummaryValue::Byte(i8::from(raid.started))),
                entry("active", NbtSummaryValue::Byte(i8::from(raid.active))),
                entry("ticks_active", NbtSummaryValue::Long(raid.ticks_active)),
                entry(
                    "raid_omen_level",
                    NbtSummaryValue::Int(raid.raid_omen_level),
                ),
                entry("groups_spawned", NbtSummaryValue::Int(raid.groups_spawned)),
                entry("cooldown_ticks", NbtSummaryValue::Int(raid.cooldown_ticks)),
                entry(
                    "post_raid_ticks",
                    NbtSummaryValue::Int(raid.post_raid_ticks),
                ),
                entry("total_health", NbtSummaryValue::Float(health.to_bits())),
                entry("group_count", NbtSummaryValue::Int(raid.group_count)),
                entry("status", NbtSummaryValue::string(status)),
                entry(
                    "center",
                    NbtSummaryValue::IntArray(vec![raid.center.x, raid.center.y, raid.center.z]),
                ),
                entry("heroes_of_the_village", NbtSummaryValue::List(heroes)),
            ])
        })
        .collect();
    let mut fields = Vec::new();
    if !data.raids.is_empty() {
        fields.push(entry("raids", NbtSummaryValue::List(raids)));
    }
    fields.push(entry("next_id", NbtSummaryValue::Int(data.next_id)));
    fields.push(entry("tick", NbtSummaryValue::Int(data.tick)));
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("data", NbtSummaryValue::Compound(fields)),
        ],
    )
}

#[cfg(test)]
mod tests;
