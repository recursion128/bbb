use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

use crate::{
    level_file::read_gzip_compound,
    schema_support::{compound_or_empty, int_or, require_java_26_1_root},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Java26WanderingTraderData {
    pub spawn_delay: i32,
    pub spawn_chance: i32,
}

impl Default for Java26WanderingTraderData {
    fn default() -> Self {
        Self {
            spawn_delay: 24000,
            spawn_chance: 25,
        }
    }
}

pub(crate) fn wandering_trader_dat(paths: &WorldSavePaths) -> std::path::PathBuf {
    paths
        .saved_data_dir()
        .join("minecraft")
        .join("wandering_trader.dat")
}

pub fn load_java_26_1_wandering_trader(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Option<Java26WanderingTraderData>> {
    let path = wandering_trader_dat(paths);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(crate::atomic_file::io_error(
                "reading wandering trader metadata",
                &path,
                e,
            ))
        }
        Ok(_) => {}
    }
    let (document, _) = read_gzip_compound(&path, limits)?;
    let root = require_java_26_1_root(&document, "wandering_trader")?;
    let data = compound_or_empty(root, "data");
    Ok(Some(Java26WanderingTraderData {
        spawn_delay: int_or(data, "spawn_delay", 24000),
        spawn_chance: int_or(data, "spawn_chance", 25),
    }))
}

/// `WanderingTraderData.java:9-18`.
pub fn java_26_1_wandering_trader_document(data: Java26WanderingTraderData) -> NamedCompound {
    let mut fields = Vec::new();
    for (key, value, default) in [
        ("spawn_delay", data.spawn_delay, 24000),
        ("spawn_chance", data.spawn_chance, 25),
    ] {
        if value != default {
            fields.push(NbtSummaryEntry {
                name: key.into(),
                value: NbtSummaryValue::Int(value),
            });
        }
    }
    NamedCompound::new(
        "",
        vec![
            NbtSummaryEntry {
                name: "DataVersion".into(),
                value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
            },
            NbtSummaryEntry {
                name: "data".into(),
                value: NbtSummaryValue::Compound(fields),
            },
        ],
    )
}

#[cfg(test)]
mod tests;
