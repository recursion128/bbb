use crate::atomic_file::io_error;
use crate::level_file::read_gzip_compound;
use crate::schema_support::{
    bool_or, compound_or_empty, field, int_array, int_or, int_value, list_or_empty,
    require_java_26_1_root,
};
use bbb_protocol::{
    nbt_summary::entry, normalize_resource_location, validate_component_nbt, NbtSummaryEntry,
    NbtSummaryValue,
};
use bbb_server_types::save::{
    NamedCompound, SaveError, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const MAP_COLOR_COUNT: usize = 128 * 128;
const DYE_COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26MapBanner {
    pub pos: [i32; 3],
    pub color: String,
    pub name: Option<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26MapFrame {
    pub pos: [i32; 3],
    pub rotation: i32,
    pub entity_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26MapSavedData {
    pub dimension: String,
    pub center_x: i32,
    pub center_z: i32,
    pub scale: i8,
    pub colors: Vec<u8>,
    pub tracking_position: bool,
    pub unlimited_tracking: bool,
    pub locked: bool,
    pub banners: Vec<Java26MapBanner>,
    pub frames: Vec<Java26MapFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26LoadedMaps {
    pub maps: BTreeMap<i32, Java26MapSavedData>,
    pub last_id: i32,
}

/// Discovers the `maps/<signed id>.dat` ids without opening individual map
/// files, and reads `maps/last_id.dat` (`MapIndex.java:12-14`).
pub fn discover_java_26_1_maps(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<(BTreeSet<i32>, i32)> {
    let directory = paths.maps_dir();
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok((BTreeSet::new(), -1));
        }
        Err(source) => return Err(io_error("listing saved maps", &directory, source)),
    };
    let mut ids = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|source| io_error("listing saved maps", &directory, source))?;
        if let Some(id) = entry.file_name().to_str().and_then(map_id) {
            ids.insert(id);
        }
    }
    let index = paths.map_index_dat();
    let last_id = if let Some((document, _)) =
        crate::level_file::read_optional_gzip_compound(&index, limits)?
    {
        // MapIndex.java:12-14.
        int_or(
            compound_or_empty(require_java_26_1_root(&document, "map_index")?, "data"),
            "map",
            -1,
        )
    } else {
        -1
    };
    Ok((ids, last_id))
}

/// Reads and parses a single `maps/<signed id>.dat` document on demand.
/// A missing file or a map the codec cannot decode answers `Ok(None)`.
pub fn load_java_26_1_map(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
    id: i32,
) -> SaveResult<Option<Java26MapSavedData>> {
    let path = paths.map_data_dat(id);
    let (document, _) = match read_gzip_compound(&path, limits) {
        Ok(loaded) => loaded,
        Err(SaveError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    parse_map_document(&document)
}

/// Discovers and loads the `maps/<signed id>.dat` family selected by
/// `MapId.key` (`MapId.java:22-23`) and `MapItemSavedData.type`
/// (`MapItemSavedData.java:79-83`). A map the codec cannot decode is absent,
/// as `SavedDataStorage.readSavedData` answers null for it.
pub fn load_java_26_1_maps(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Java26LoadedMaps> {
    let (ids, last_id) = discover_java_26_1_maps(paths, limits)?;
    let mut maps = BTreeMap::new();
    for id in ids {
        if let Some(map) = load_java_26_1_map(paths, limits, id)? {
            maps.insert(id, map);
        }
    }
    Ok(Java26LoadedMaps { maps, last_id })
}

fn map_id(name: &str) -> Option<i32> {
    let stem = name.strip_suffix(".dat")?;
    let id = stem.parse::<i32>().ok()?;
    (stem == id.to_string()).then_some(id)
}

// MapItemSavedData.java:60-74.
fn parse_map_document(document: &NamedCompound) -> SaveResult<Option<Java26MapSavedData>> {
    let data = compound_or_empty(require_java_26_1_root(document, "map_saved_data")?, "data");
    Ok(parse_map_data(data))
}

fn parse_map_data(data: &[NbtSummaryEntry]) -> Option<Java26MapSavedData> {
    let Some(NbtSummaryValue::String(dimension)) = field(data, "dimension") else {
        return None;
    };
    let dimension = dimension
        .try_to_string()
        .ok()
        .and_then(|raw| normalize_resource_location(raw).ok())?;
    let NbtSummaryValue::ByteArray(raw_colors) = field(data, "colors")? else {
        return None;
    };
    let colors = if raw_colors.len() == MAP_COLOR_COUNT {
        raw_colors.iter().map(|value| *value as u8).collect()
    } else {
        vec![0; MAP_COLOR_COUNT]
    };
    Some(Java26MapSavedData {
        dimension,
        center_x: field(data, "xCenter").and_then(int_value)?,
        center_z: field(data, "zCenter").and_then(int_value)?,
        scale: int_or(data, "scale", 0).clamp(0, 4) as i8,
        colors,
        tracking_position: bool_or(data, "trackingPosition", true),
        unlimited_tracking: bool_or(data, "unlimitedTracking", false),
        locked: bool_or(data, "locked", false),
        banners: list_or_empty(data, "banners")
            .iter()
            .filter_map(parse_banner)
            .collect(),
        frames: list_or_empty(data, "frames")
            .iter()
            .filter_map(parse_frame)
            .collect(),
    })
}

// MapBanner.java:16-22.
fn parse_banner(value: &NbtSummaryValue) -> Option<Java26MapBanner> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let color = match field(entries, "color") {
        Some(NbtSummaryValue::String(color)) => DYE_COLORS
            .iter()
            .find(|known| color == **known)
            .unwrap_or(&"white")
            .to_string(),
        _ => "white".to_string(),
    };
    let name = field(entries, "name")
        .filter(|value| validate_component_nbt(value, "map_saved_data.banners.name").is_ok())
        .cloned();
    Some(Java26MapBanner {
        pos: field(entries, "pos").and_then(int_array)?,
        color,
        name,
    })
}

// MapFrame.java:8-14.
fn parse_frame(value: &NbtSummaryValue) -> Option<Java26MapFrame> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    Some(Java26MapFrame {
        pos: field(entries, "pos").and_then(int_array)?,
        rotation: field(entries, "rotation").and_then(int_value)?,
        entity_id: field(entries, "entity_id").and_then(int_value)?,
    })
}

// MapIndex.java:12-14; SavedDataStorage.java:198-204: the codec omits its -1 default.
pub(crate) fn java_26_1_map_index_document(last_id: i32) -> NamedCompound {
    let data = if last_id == -1 {
        Vec::new()
    } else {
        vec![entry("map", NbtSummaryValue::Int(last_id))]
    };
    NamedCompound::new(
        "",
        vec![
            entry("data", NbtSummaryValue::Compound(data)),
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
        ],
    )
}

/// `SavedDataStorage.encodeUnchecked` document for one map
/// (`SavedDataStorage.java:198-204`).
pub fn java_26_1_map_document(map: &Java26MapSavedData) -> NamedCompound {
    let mut data = vec![
        entry("dimension", NbtSummaryValue::string(map.dimension.clone())),
        entry("xCenter", NbtSummaryValue::Int(map.center_x)),
        entry("zCenter", NbtSummaryValue::Int(map.center_z)),
    ];
    if map.scale != 0 {
        data.push(entry("scale", NbtSummaryValue::Byte(map.scale)));
    }
    data.push(entry(
        "colors",
        NbtSummaryValue::ByteArray(map.colors.iter().map(|value| *value as i8).collect()),
    ));
    if !map.tracking_position {
        data.push(entry("trackingPosition", NbtSummaryValue::Byte(0)));
    }
    if map.unlimited_tracking {
        data.push(entry("unlimitedTracking", NbtSummaryValue::Byte(1)));
    }
    if map.locked {
        data.push(entry("locked", NbtSummaryValue::Byte(1)));
    }
    if !map.banners.is_empty() {
        data.push(entry(
            "banners",
            NbtSummaryValue::List(map.banners.iter().map(encode_banner).collect()),
        ));
    }
    if !map.frames.is_empty() {
        data.push(entry(
            "frames",
            NbtSummaryValue::List(map.frames.iter().map(encode_frame).collect()),
        ));
    }
    NamedCompound::new(
        "",
        vec![
            entry("data", NbtSummaryValue::Compound(data)),
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
        ],
    )
}

fn encode_banner(banner: &Java26MapBanner) -> NbtSummaryValue {
    let mut entries = vec![entry("pos", NbtSummaryValue::IntArray(banner.pos.to_vec()))];
    if banner.color != "white" {
        entries.push(entry(
            "color",
            NbtSummaryValue::string(banner.color.clone()),
        ));
    }
    if let Some(name) = &banner.name {
        entries.push(entry("name", name.clone()));
    }
    NbtSummaryValue::Compound(entries)
}

fn encode_frame(frame: &Java26MapFrame) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("pos", NbtSummaryValue::IntArray(frame.pos.to_vec())),
        entry("rotation", NbtSummaryValue::Int(frame.rotation)),
        entry("entity_id", NbtSummaryValue::Int(frame.entity_id)),
    ])
}

#[cfg(test)]
mod tests;
