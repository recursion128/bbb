//! `Stopwatches.TYPE` (`Stopwatches.java:16-22`): `data/minecraft/stopwatches.dat`
//! holding `{stopwatches: {<id>: <elapsed ms>}}`.

use std::collections::BTreeMap;

use bbb_protocol::{nbt_summary::entry, normalize_resource_location, NbtSummaryValue};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

use crate::schema_support::{compound_or_empty, long_value, require_java_26_1_root};

/// Stopwatch id to accumulated elapsed milliseconds.
pub type Java26Stopwatches = BTreeMap<String, i64>;

pub(crate) fn stopwatches_relative_path() -> std::path::PathBuf {
    std::path::PathBuf::from("data")
        .join("minecraft")
        .join("stopwatches.dat")
}

pub fn load_java_26_1_stopwatches(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Java26Stopwatches> {
    let path = paths.root().join(stopwatches_relative_path());
    let Some((document, _)) = crate::level_file::read_optional_gzip_compound(&path, limits)? else {
        return Ok(Java26Stopwatches::new());
    };
    let data = compound_or_empty(require_java_26_1_root(&document, "stopwatches")?, "data");
    Ok(compound_or_empty(data, "stopwatches")
        .iter()
        .filter_map(|entry| {
            Some((
                normalize_resource_location(entry.name.try_to_string().ok()?).ok()?,
                long_value(&entry.value)?,
            ))
        })
        .collect())
}

pub fn java_26_1_stopwatches_document(stopwatches: &Java26Stopwatches) -> NamedCompound {
    let entries = stopwatches
        .iter()
        .map(|(id, elapsed)| entry(id, NbtSummaryValue::Long(*elapsed)))
        .collect();
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry(
                "data",
                NbtSummaryValue::Compound(vec![entry(
                    "stopwatches",
                    NbtSummaryValue::Compound(entries),
                )]),
            ),
        ],
    )
}
