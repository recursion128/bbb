//! `CustomBossEvents` saved data (`CustomBossEvents.java:19-27`).

use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use crate::level_file::encode_gzip_compound;
#[cfg(test)]
use crate::level_file::encode_gzip_named_compound;
use crate::player_schema::field_parsers::number_i32;
use crate::schema_support::{field, require_java_26_1_root, uuid_nbt, uuid_value};
use bbb_protocol::{nbt_summary::entry, normalize_resource_location, NbtSummaryValue};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Java26CustomBossEventsPacked(pub BTreeMap<String, Java26CustomBossEventPacked>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26CustomBossEventPacked {
    pub name: NbtSummaryValue,
    pub visible: bool,
    pub value: i32,
    pub max: i32,
    pub color: String,
    pub overlay: String,
    pub darken_screen: bool,
    pub play_boss_music: bool,
    pub create_world_fog: bool,
    pub players: Vec<Uuid>,
}

impl Default for Java26CustomBossEventPacked {
    fn default() -> Self {
        Self {
            name: NbtSummaryValue::string(String::new()),
            visible: false,
            value: 0,
            max: 100,
            color: "white".into(),
            overlay: "progress".into(),
            darken_screen: false,
            play_boss_music: false,
            create_world_fog: false,
            players: Vec::new(),
        }
    }
}

fn path(paths: &WorldSavePaths) -> std::path::PathBuf {
    paths
        .saved_data_dir()
        .join("minecraft")
        .join("custom_boss_events.dat")
}

pub fn java_26_1_custom_boss_events_document(
    packed: &Java26CustomBossEventsPacked,
) -> NamedCompound {
    let mut events = Vec::with_capacity(packed.0.len());
    for (id, row) in &packed.0 {
        let mut fields = vec![entry("Name", row.name.clone())];
        for (name, value, default) in [
            ("Visible", row.visible, false),
            ("DarkenScreen", row.darken_screen, false),
            ("PlayBossMusic", row.play_boss_music, false),
            ("CreateWorldFog", row.create_world_fog, false),
        ] {
            if value != default {
                fields.push(entry(name, NbtSummaryValue::Byte(1)));
            }
        }
        if row.value != 0 {
            fields.push(entry("Value", NbtSummaryValue::Int(row.value)));
        }
        if row.max != 100 {
            fields.push(entry("Max", NbtSummaryValue::Int(row.max)));
        }
        if row.color != "white" {
            fields.push(entry("Color", NbtSummaryValue::string(row.color.clone())));
        }
        if row.overlay != "progress" {
            fields.push(entry(
                "Overlay",
                NbtSummaryValue::string(row.overlay.clone()),
            ));
        }
        let players = row.players.iter().copied().collect::<BTreeSet<_>>();
        if !players.is_empty() {
            fields.push(entry(
                "Players",
                NbtSummaryValue::List(players.into_iter().map(uuid_nbt).collect()),
            ));
        }
        events.push(entry(id, NbtSummaryValue::Compound(fields)));
    }
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("data", NbtSummaryValue::Compound(events)),
        ],
    )
}

pub fn load_java_26_1_custom_boss_events(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Java26CustomBossEventsPacked> {
    let file = path(paths);
    let Some((doc, _)) = crate::level_file::read_optional_gzip_compound(&file, limits)? else {
        return Ok(Default::default());
    };
    let root = require_java_26_1_root(&doc, "custom_boss_events")?;
    let Some(NbtSummaryValue::Compound(data)) = field(root, "data") else {
        return Ok(Default::default());
    };
    // `Packed.CODEC` under `resultOrPartial`: unknown fields are ignored, an
    // unreadable optional field takes its default, and a bar without a name or
    // with an invalid id is dropped.
    let mut out = BTreeMap::new();
    for entry in data {
        let Some(id) = entry
            .name
            .try_to_string()
            .ok()
            .and_then(|raw| normalize_resource_location(raw).ok())
        else {
            continue;
        };
        let NbtSummaryValue::Compound(row) = &entry.value else {
            continue;
        };
        let Some(name) = field(row, "Name") else {
            continue;
        };
        let int = |name: &str| field(row, name).and_then(number_i32);
        let flag = |name: &str| int(name).is_some_and(|value| value != 0);
        let string_in = |name: &str, allowed: &[&str], default: &str| match field(row, name) {
            Some(NbtSummaryValue::String(value)) => allowed
                .iter()
                .find(|known| value == **known)
                .copied()
                .unwrap_or(default)
                .to_string(),
            _ => default.to_string(),
        };
        let players = match field(row, "Players") {
            Some(NbtSummaryValue::List(values)) => values
                .iter()
                .filter_map(uuid_value)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            _ => Vec::new(),
        };
        out.insert(
            id,
            Java26CustomBossEventPacked {
                name: name.clone(),
                visible: flag("Visible"),
                value: int("Value").unwrap_or(0),
                max: int("Max").unwrap_or(100),
                color: string_in(
                    "Color",
                    &["pink", "blue", "red", "green", "yellow", "purple", "white"],
                    "white",
                ),
                overlay: string_in(
                    "Overlay",
                    &[
                        "progress",
                        "notched_6",
                        "notched_10",
                        "notched_12",
                        "notched_20",
                    ],
                    "progress",
                ),
                darken_screen: flag("DarkenScreen"),
                play_boss_music: flag("PlayBossMusic"),
                create_world_fog: flag("CreateWorldFog"),
                players,
            },
        );
    }
    Ok(Java26CustomBossEventsPacked(out))
}

pub fn save_java_26_1_custom_boss_events(
    paths: &WorldSavePaths,
    packed: &Java26CustomBossEventsPacked,
    limits: WorldSaveLimits,
) -> SaveResult<Vec<u8>> {
    encode_gzip_compound(
        &path(paths),
        &java_26_1_custom_boss_events_document(packed),
        limits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_omitted() {
        let mut map = BTreeMap::new();
        map.insert(
            "minecraft:test".into(),
            Java26CustomBossEventPacked {
                name: bbb_protocol::component_literal_nbt("bar"),
                ..Default::default()
            },
        );
        let doc = java_26_1_custom_boss_events_document(&Java26CustomBossEventsPacked(map));
        let NbtSummaryValue::Compound(root) = doc.to_summary() else {
            panic!()
        };
        let NbtSummaryValue::Compound(data) =
            &root.iter().find(|e| e.name == "data").unwrap().value
        else {
            panic!()
        };
        let NbtSummaryValue::Compound(row) = &data[0].value else {
            panic!()
        };
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].name, "Name");
    }

    #[test]
    fn codec_set_membership_is_deduplicated_on_load() {
        let root = std::env::temp_dir().join(format!(
            "bbb-custom-boss-events-codec-set-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let paths = WorldSavePaths::new(&root).unwrap();
        std::fs::create_dir_all(paths.saved_data_dir().join("minecraft")).unwrap();
        let player = Uuid::from_u128(7);
        let document = NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry(
                    "data",
                    NbtSummaryValue::Compound(vec![entry(
                        "minecraft:test",
                        NbtSummaryValue::Compound(vec![
                            entry("Name", bbb_protocol::component_literal_nbt("Test")),
                            entry(
                                "Players",
                                NbtSummaryValue::List(vec![uuid_nbt(player), uuid_nbt(player)]),
                            ),
                        ]),
                    )]),
                ),
            ],
        );
        let bytes =
            encode_gzip_named_compound(&path(&paths), &document, Default::default()).unwrap();
        std::fs::write(path(&paths), bytes).unwrap();

        let loaded = load_java_26_1_custom_boss_events(&paths, Default::default()).unwrap();
        assert_eq!(loaded.0["minecraft:test"].players, [player]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
