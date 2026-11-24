//! `ScoreboardSaveData.TYPE` (`ScoreboardSaveData.java:13-18`) on disk.
//!
//! `SavedDataStorage.getDataFile` (`SavedDataStorage.java:56-63`) resolves
//! `minecraft:scoreboard` against the level data folder
//! (`MinecraftServer.java:330`, `MinecraftServer.java:434`), so the file is
//! `<world>/data/minecraft/scoreboard.dat`. `encodeUnchecked`
//! (`SavedDataStorage.java:198-204`) puts `Packed.CODEC`
//! (`ScoreboardSaveData.java:44-52`) under `data` beside `DataVersion`.
//! `Packed.EMPTY` (`ScoreboardSaveData.java:43`) encodes as `{}`.

use crate::schema_support::{bool_or, field, int_value, require_java_26_1_root};
use bbb_facts::scoreboard_team_options::{
    TeamCollisionRule, TeamVisibility, VANILLA_TEAM_COLLISION_RULES, VANILLA_TEAM_VISIBILITIES,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::chat_formattings::{ChatFormatting, VANILLA_CHAT_FORMATTINGS};
use bbb_protocol::entity_types::vanilla_entity_type_for_resource_id;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::objective_render_types::{ObjectiveRenderType, VANILLA_OBJECTIVE_RENDER_TYPES};
use bbb_protocol::scoreboard_display_slots::{
    ScoreboardDisplaySlot, VANILLA_SCOREBOARD_DISPLAY_SLOTS,
};
use bbb_protocol::stat_types::{
    vanilla_custom_stat_by_resource_id, vanilla_stat_type_by_resource_id,
    VANILLA_OBJECTIVE_CRITERIA_NAMES,
};
use bbb_protocol::{
    nbt_summary::entry, normalize_resource_location, JavaString, NbtSummaryEntry, NbtSummaryValue,
    RichTextStyle,
};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

const SCHEMA: &str = "scoreboard";
const DUMMY_CRITERIA: &str = "dummy";
const STYLE_FIELDS: &[&str] = &[
    "color",
    "shadow_color",
    "bold",
    "italic",
    "underlined",
    "strikethrough",
    "obfuscated",
    "click_event",
    "hover_event",
    "insertion",
    "font",
];

/// `ScoreboardSaveData.Packed` (`ScoreboardSaveData.java:40-52`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Java26ScoreboardPacked {
    pub objectives: Vec<Java26ScoreboardObjective>,
    pub scores: Vec<Java26ScoreboardPackedScore>,
    pub display_slots: Vec<(ScoreboardDisplaySlot, JavaString)>,
    pub teams: Vec<Java26ScoreboardTeam>,
}

/// `Objective.Packed` (`Objective.java:112-130`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26ScoreboardObjective {
    pub name: JavaString,
    pub criteria: String,
    pub display_name: NbtSummaryValue,
    pub render_type: ObjectiveRenderType,
    pub display_auto_update: bool,
    pub format: Option<Java26ScoreboardNumberFormat>,
}

/// `Scoreboard.PackedScore` plus flattened `Score.Packed`
/// (`Scoreboard.java:424-432`, `Score.java:68-77`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26ScoreboardPackedScore {
    pub owner: JavaString,
    pub objective: JavaString,
    pub score: i32,
    pub locked: bool,
    pub display: Option<NbtSummaryValue>,
    pub format: Option<Java26ScoreboardNumberFormat>,
}

/// `PlayerTeam.Packed` (`PlayerTeam.java:208-236`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26ScoreboardTeam {
    pub name: JavaString,
    pub display_name: Option<NbtSummaryValue>,
    pub color: Option<ChatFormatting>,
    pub allow_friendly_fire: bool,
    pub see_friendly_invisibles: bool,
    pub member_name_prefix: Option<NbtSummaryValue>,
    pub member_name_suffix: Option<NbtSummaryValue>,
    pub name_tag_visibility: TeamVisibility,
    pub death_message_visibility: TeamVisibility,
    pub collision_rule: TeamCollisionRule,
    pub players: Vec<JavaString>,
}

/// `NumberFormatTypes.CODEC` (`NumberFormatTypes.java:14-17`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Java26ScoreboardNumberFormat {
    Blank,
    Styled(NbtSummaryValue),
    Fixed(NbtSummaryValue),
}

fn scoreboard_dat(paths: &WorldSavePaths) -> std::path::PathBuf {
    paths
        .saved_data_dir()
        .join("minecraft")
        .join("scoreboard.dat")
}

/// The `minecraft:scoreboard` saved data one save carries.
///
/// A missing file is `Packed.EMPTY`, exactly as
/// `savedDataStorage.computeIfAbsent(ScoreboardSaveData.TYPE)`
/// (`MinecraftServer.java:434`) builds one.
pub fn load_java_26_1_scoreboard(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Java26ScoreboardPacked> {
    load_java_26_1_scoreboard_with_registry(
        paths,
        limits,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`load_java_26_1_scoreboard`].
pub fn load_java_26_1_scoreboard_with_registry(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> SaveResult<Java26ScoreboardPacked> {
    let path = scoreboard_dat(paths);
    let Some((document, _)) = crate::level_file::read_optional_gzip_compound(&path, limits)? else {
        return Ok(Java26ScoreboardPacked::default());
    };
    parse_java_26_1_scoreboard_document(&document, registry)
}

/// The `data/minecraft/scoreboard.dat` document one save is written with,
/// the inverse of [`load_java_26_1_scoreboard`].
pub fn java_26_1_scoreboard_document(packed: &Java26ScoreboardPacked) -> NamedCompound {
    saved_data_document(encode_packed(packed))
}

/// `Packed.CODEC` read the way `SavedDataStorage` reads it
/// (`resultOrPartial`): unknown fields are ignored, an unreadable optional field
/// takes its default, and an entry missing a required field is dropped.
fn parse_java_26_1_scoreboard_document(
    document: &NamedCompound,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> SaveResult<Java26ScoreboardPacked> {
    let entries = require_java_26_1_root(document, SCHEMA)?;
    let Some(NbtSummaryValue::Compound(data)) = field(entries, "data") else {
        return Ok(Java26ScoreboardPacked::default());
    };
    Ok(Java26ScoreboardPacked {
        objectives: parse_list(data, "Objectives", |value| parse_objective(value, registry)),
        scores: parse_list(data, "PlayerScores", parse_score),
        display_slots: field(data, "DisplaySlots")
            .map(parse_display_slots)
            .unwrap_or_default(),
        teams: parse_list(data, "Teams", parse_team),
    })
}

fn encode_packed(packed: &Java26ScoreboardPacked) -> Vec<NbtSummaryEntry> {
    let mut data = Vec::new();
    push_encoded_list(
        &mut data,
        "Objectives",
        &packed.objectives,
        encode_objective,
    );
    push_encoded_list(&mut data, "PlayerScores", &packed.scores, encode_score);
    if !packed.display_slots.is_empty() {
        data.push(entry(
            "DisplaySlots",
            encode_display_slots(&packed.display_slots),
        ));
    }
    push_encoded_list(&mut data, "Teams", &packed.teams, encode_team);
    data
}

fn parse_objective(
    value: &NbtSummaryValue,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<Java26ScoreboardObjective> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    Some(Java26ScoreboardObjective {
        name: string(fields, "Name")?.clone(),
        criteria: string(fields, "CriteriaName")
            .and_then(|name| name.try_to_string().ok())
            .as_deref()
            .and_then(|name| canonical_objective_criteria_with_registry(name, registry))
            .unwrap_or_else(|| DUMMY_CRITERIA.to_string()),
        display_name: field(fields, "DisplayName")?.clone(),
        render_type: string(fields, "RenderType")
            .and_then(|name| {
                VANILLA_OBJECTIVE_RENDER_TYPES
                    .iter()
                    .find(|entry| name == entry.serialized_name)
            })
            .map_or(ObjectiveRenderType::Integer, |entry| entry.render_type),
        display_auto_update: bool_or(fields, "display_auto_update", false),
        format: field(fields, "format").and_then(parse_number_format),
    })
}

fn encode_objective(objective: &Java26ScoreboardObjective) -> NbtSummaryValue {
    let mut fields = vec![entry(
        "Name",
        NbtSummaryValue::string(objective.name.clone()),
    )];
    if objective.criteria != DUMMY_CRITERIA {
        fields.push(entry(
            "CriteriaName",
            NbtSummaryValue::string(objective.criteria.clone()),
        ));
    }
    fields.push(entry("DisplayName", objective.display_name.clone()));
    if objective.render_type != ObjectiveRenderType::Integer {
        fields.push(entry(
            "RenderType",
            NbtSummaryValue::string(objective.render_type.as_str()),
        ));
    }
    if objective.display_auto_update {
        fields.push(entry("display_auto_update", NbtSummaryValue::Byte(1)));
    }
    if let Some(format) = &objective.format {
        fields.push(entry("format", encode_number_format(format)));
    }
    NbtSummaryValue::Compound(fields)
}

fn parse_score(value: &NbtSummaryValue) -> Option<Java26ScoreboardPackedScore> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    Some(Java26ScoreboardPackedScore {
        owner: string(fields, "Name")?.clone(),
        objective: string(fields, "Objective")?.clone(),
        score: field(fields, "Score").and_then(int_value).unwrap_or(0),
        locked: bool_or(fields, "Locked", false),
        display: field(fields, "display").cloned(),
        format: field(fields, "format").and_then(parse_number_format),
    })
}

fn encode_score(score: &Java26ScoreboardPackedScore) -> NbtSummaryValue {
    let mut fields = vec![
        entry("Name", NbtSummaryValue::string(score.owner.clone())),
        entry(
            "Objective",
            NbtSummaryValue::string(score.objective.clone()),
        ),
    ];
    if score.score != 0 {
        fields.push(entry("Score", NbtSummaryValue::Int(score.score)));
    }
    if score.locked {
        fields.push(entry("Locked", NbtSummaryValue::Byte(1)));
    }
    if let Some(display) = &score.display {
        fields.push(entry("display", display.clone()));
    }
    if let Some(format) = &score.format {
        fields.push(entry("format", encode_number_format(format)));
    }
    NbtSummaryValue::Compound(fields)
}

fn parse_team(value: &NbtSummaryValue) -> Option<Java26ScoreboardTeam> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let visibility = |name| {
        string(fields, name)
            .and_then(|name| {
                VANILLA_TEAM_VISIBILITIES
                    .iter()
                    .find(|entry| name == entry.serialized_name)
            })
            .map_or(TeamVisibility::Always, |entry| entry.visibility)
    };
    let affix = |name| match field(fields, name) {
        Some(NbtSummaryValue::String(text)) if text.is_empty() => None,
        other => other.cloned(),
    };
    Some(Java26ScoreboardTeam {
        name: string(fields, "Name")?.clone(),
        display_name: field(fields, "DisplayName").cloned(),
        color: string(fields, "TeamColor").and_then(|name| {
            VANILLA_CHAT_FORMATTINGS
                .iter()
                .find(|entry| name == entry.serialized_name && !entry.is_format)
                .map(|entry| entry.formatting)
        }),
        allow_friendly_fire: bool_or(fields, "AllowFriendlyFire", true),
        see_friendly_invisibles: bool_or(fields, "SeeFriendlyInvisibles", true),
        member_name_prefix: affix("MemberNamePrefix"),
        member_name_suffix: affix("MemberNameSuffix"),
        name_tag_visibility: visibility("NameTagVisibility"),
        death_message_visibility: visibility("DeathMessageVisibility"),
        collision_rule: string(fields, "CollisionRule")
            .and_then(|name| {
                VANILLA_TEAM_COLLISION_RULES
                    .iter()
                    .find(|entry| name == entry.serialized_name)
            })
            .map_or(TeamCollisionRule::Always, |entry| entry.collision_rule),
        players: match field(fields, "Players") {
            Some(NbtSummaryValue::List(values)) => values
                .iter()
                .filter_map(|value| match value {
                    NbtSummaryValue::String(player) => Some(player.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        },
    })
}

fn encode_team(team: &Java26ScoreboardTeam) -> NbtSummaryValue {
    let mut fields = vec![entry("Name", NbtSummaryValue::string(team.name.clone()))];
    if let Some(display_name) = &team.display_name {
        fields.push(entry("DisplayName", display_name.clone()));
    }
    if let Some(color) = team.color {
        fields.push(entry("TeamColor", NbtSummaryValue::string(color.as_str())));
    }
    if !team.allow_friendly_fire {
        fields.push(entry("AllowFriendlyFire", NbtSummaryValue::Byte(0)));
    }
    if !team.see_friendly_invisibles {
        fields.push(entry("SeeFriendlyInvisibles", NbtSummaryValue::Byte(0)));
    }
    if let Some(prefix) = &team.member_name_prefix {
        fields.push(entry("MemberNamePrefix", prefix.clone()));
    }
    if let Some(suffix) = &team.member_name_suffix {
        fields.push(entry("MemberNameSuffix", suffix.clone()));
    }
    if team.name_tag_visibility != TeamVisibility::Always {
        fields.push(entry(
            "NameTagVisibility",
            NbtSummaryValue::string(team.name_tag_visibility.as_str()),
        ));
    }
    if team.death_message_visibility != TeamVisibility::Always {
        fields.push(entry(
            "DeathMessageVisibility",
            NbtSummaryValue::string(team.death_message_visibility.as_str()),
        ));
    }
    if team.collision_rule != TeamCollisionRule::Always {
        fields.push(entry(
            "CollisionRule",
            NbtSummaryValue::string(team.collision_rule.as_str()),
        ));
    }
    if !team.players.is_empty() {
        fields.push(entry(
            "Players",
            NbtSummaryValue::List(
                team.players
                    .iter()
                    .map(|player| NbtSummaryValue::string(player.clone()))
                    .collect(),
            ),
        ));
    }
    NbtSummaryValue::Compound(fields)
}

fn parse_display_slots(value: &NbtSummaryValue) -> Vec<(ScoreboardDisplaySlot, JavaString)> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Vec::new();
    };
    let mut slots: Vec<(ScoreboardDisplaySlot, JavaString)> = Vec::new();
    for field in fields {
        let Some(slot) = VANILLA_SCOREBOARD_DISPLAY_SLOTS
            .iter()
            .find(|entry| field.name == entry.serialized_name)
        else {
            continue;
        };
        let NbtSummaryValue::String(name) = &field.value else {
            continue;
        };
        slots.retain(|(existing, _)| *existing != slot.slot);
        slots.push((slot.slot, name.clone()));
    }
    slots.sort_by_key(|(slot, _)| slot.id());
    slots
}

fn encode_display_slots(slots: &[(ScoreboardDisplaySlot, JavaString)]) -> NbtSummaryValue {
    let fields = VANILLA_SCOREBOARD_DISPLAY_SLOTS
        .iter()
        .filter_map(|catalog| {
            let (_, name) = slots.iter().find(|(slot, _)| *slot == catalog.slot)?;
            Some(entry(
                catalog.serialized_name,
                NbtSummaryValue::string(name.clone()),
            ))
        })
        .collect();
    NbtSummaryValue::Compound(fields)
}

fn parse_number_format(value: &NbtSummaryValue) -> Option<Java26ScoreboardNumberFormat> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    match normalize_resource_location(string(fields, "type")?.try_to_string().ok()?)
        .ok()?
        .as_str()
    {
        "minecraft:blank" => Some(Java26ScoreboardNumberFormat::Blank),
        "minecraft:styled" => {
            let style = NbtSummaryValue::Compound(
                fields
                    .iter()
                    .filter(|field| STYLE_FIELDS.iter().any(|name| field.name == *name))
                    .cloned()
                    .collect(),
            );
            RichTextStyle::try_from_style_nbt(style.clone()).ok()?;
            Some(Java26ScoreboardNumberFormat::Styled(style))
        }
        "minecraft:fixed" => Some(Java26ScoreboardNumberFormat::Fixed(
            field(fields, "value")?.clone(),
        )),
        _ => None,
    }
}

fn encode_number_format(format: &Java26ScoreboardNumberFormat) -> NbtSummaryValue {
    match format {
        Java26ScoreboardNumberFormat::Blank => {
            NbtSummaryValue::Compound(vec![format_type("minecraft:blank")])
        }
        Java26ScoreboardNumberFormat::Styled(style) => {
            let mut fields = vec![format_type("minecraft:styled")];
            if let NbtSummaryValue::Compound(style_fields) = style {
                fields.extend(style_fields.iter().cloned());
            }
            NbtSummaryValue::Compound(fields)
        }
        Java26ScoreboardNumberFormat::Fixed(value) => NbtSummaryValue::Compound(vec![
            format_type("minecraft:fixed"),
            entry("value", value.clone()),
        ]),
    }
}

fn parse_list<T>(
    fields: &[NbtSummaryEntry],
    name: &str,
    parse: impl Fn(&NbtSummaryValue) -> Option<T>,
) -> Vec<T> {
    match field(fields, name) {
        Some(NbtSummaryValue::List(values)) => values.iter().filter_map(parse).collect(),
        _ => Vec::new(),
    }
}

fn string<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a JavaString> {
    match field(fields, name)? {
        NbtSummaryValue::String(value) => Some(value),
        _ => None,
    }
}

fn push_encoded_list<T>(
    data: &mut Vec<NbtSummaryEntry>,
    name: &str,
    values: &[T],
    encode: impl Fn(&T) -> NbtSummaryValue,
) {
    if !values.is_empty() {
        data.push(entry(
            name,
            NbtSummaryValue::List(values.iter().map(encode).collect()),
        ));
    }
}

/// `ObjectiveCriteria.byName` (`ObjectiveCriteria.java:99-111`): the criterion's
/// stored name, which for a stat is the full `Stat.buildName` form
/// (`Stat.java:27-32`) whatever short form named it.
pub fn canonical_objective_criteria(name: &str) -> Option<String> {
    canonical_objective_criteria_with_registry(
        name,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`canonical_objective_criteria`].
pub fn canonical_objective_criteria_with_registry(
    name: &str,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<String> {
    if VANILLA_OBJECTIVE_CRITERIA_NAMES.contains(&name) {
        return Some(name.to_string());
    }
    let colon = name.find(':')?;
    let stat_type_id = identifier_by_separator(&name[..colon], '.')?;
    let stat_type = vanilla_stat_type_by_resource_id(&stat_type_id)?;
    let value_id = identifier_by_separator(&name[colon + 1..], '.')?;
    let known = match stat_type.value_registry {
        "minecraft:block" => {
            vanilla_block_type_for_resource_id(&value_id).is_some()
                || registry.block_by_resource_id(&value_id).is_some()
        }
        "minecraft:item" => {
            vanilla_item_type_for_resource_id(&value_id).is_some()
                || registry.item_by_resource_id(&value_id).is_some()
        }
        "minecraft:entity_type" => {
            vanilla_entity_type_for_resource_id(&value_id).is_some()
                || registry.entity_by_resource_id(&value_id).is_some()
        }
        "minecraft:custom_stat" => vanilla_custom_stat_by_resource_id(&value_id).is_some(),
        _ => false,
    };
    known.then(|| {
        format!(
            "{}:{}",
            stat_type_id.replace(':', "."),
            value_id.replace(':', ".")
        )
    })
}

fn identifier_by_separator(value: &str, separator: char) -> Option<String> {
    let (namespace, path) = match value.find(separator) {
        Some(0) => ("minecraft", value.get(1..)?),
        Some(index) => (value.get(..index)?, value.get(index + 1..)?),
        None => ("minecraft", value),
    };
    normalize_resource_location(format!("{namespace}:{path}")).ok()
}

fn saved_data_document(data: Vec<NbtSummaryEntry>) -> NamedCompound {
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("data", NbtSummaryValue::Compound(data)),
        ],
    )
}

fn format_type(resource_id: &str) -> NbtSummaryEntry {
    entry("type", NbtSummaryValue::string(resource_id))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::level_file::encode_gzip_named_compound;

    fn scratch(name: &str) -> WorldSavePaths {
        let root =
            std::env::temp_dir().join(format!("bbb-scoreboard-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let paths = WorldSavePaths::new(&root).expect("the scratch root is a usable save path");
        fs::create_dir_all(paths.saved_data_dir().join("minecraft"))
            .expect("the data folder is creatable");
        paths
    }

    fn write(paths: &WorldSavePaths, document: &NamedCompound) {
        let limits = WorldSaveLimits::default();
        let path = scoreboard_dat(paths);
        fs::write(
            &path,
            encode_gzip_named_compound(&path, document, limits).expect("the document encodes"),
        )
        .expect("the file is writable");
    }

    fn load(paths: &WorldSavePaths) -> Java26ScoreboardPacked {
        load_java_26_1_scoreboard(paths, WorldSaveLimits::default())
            .expect("the scoreboard document loads")
    }

    fn cleanup(paths: WorldSavePaths) {
        fs::remove_dir_all(paths.root()).expect("the scratch root is removable");
    }

    fn one_objective() -> Java26ScoreboardPacked {
        Java26ScoreboardPacked {
            objectives: vec![Java26ScoreboardObjective {
                name: "kills".into(),
                criteria: DUMMY_CRITERIA.to_string(),
                display_name: NbtSummaryValue::string("Kills"),
                render_type: ObjectiveRenderType::Integer,
                display_auto_update: false,
                format: None,
            }],
            ..Java26ScoreboardPacked::default()
        }
    }

    fn one_objective_document() -> NamedCompound {
        saved_data_document(vec![entry(
            "Objectives",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("Name", NbtSummaryValue::string("kills")),
                entry("DisplayName", NbtSummaryValue::string("Kills")),
            ])]),
        )])
    }

    #[test]
    fn the_empty_scoreboard_packed_round_trips_as_an_empty_data_compound() {
        let paths = scratch("empty");
        let document = java_26_1_scoreboard_document(&Java26ScoreboardPacked::default());
        assert_eq!(document, saved_data_document(Vec::new()));
        write(&paths, &document);
        assert_eq!(load(&paths), Java26ScoreboardPacked::default());
        cleanup(paths);
    }

    #[test]
    fn an_absent_scoreboard_file_is_packed_empty() {
        let paths = scratch("absent");
        assert_eq!(load(&paths), Java26ScoreboardPacked::default());
        cleanup(paths);
    }

    #[test]
    fn a_scoreboard_with_one_objective_round_trips_writer_canonical_nbt() {
        let paths = scratch("one-objective");
        let packed = one_objective();
        let document = java_26_1_scoreboard_document(&packed);
        assert_eq!(document, one_objective_document());
        write(&paths, &document);
        assert_eq!(load(&paths), packed);
        cleanup(paths);
    }

    #[test]
    fn unknown_fields_are_ignored_and_unreadable_entries_skipped() {
        let paths = scratch("lenient");
        let mut document = saved_data_document(vec![
            entry(
                "Objectives",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Compound(vec![
                        entry("Name", NbtSummaryValue::string("kills")),
                        entry("DisplayName", NbtSummaryValue::string("Kills")),
                        entry("CriteriaName", NbtSummaryValue::string("bogus")),
                        entry("extra", NbtSummaryValue::Int(1)),
                    ]),
                    NbtSummaryValue::Compound(vec![entry(
                        "Name",
                        NbtSummaryValue::string("no_display"),
                    )]),
                ]),
            ),
            entry("bogus", NbtSummaryValue::List(Vec::new())),
        ]);
        document
            .entries_mut()
            .push(entry("extra", NbtSummaryValue::string("nope")));
        write(&paths, &document);
        assert_eq!(load(&paths), one_objective());
        cleanup(paths);
    }

    #[test]
    fn a_short_stat_criterion_loads_under_its_full_name() {
        let paths = scratch("short-criteria");
        write(
            &paths,
            &saved_data_document(vec![entry(
                "Objectives",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry("Name", NbtSummaryValue::string("kills")),
                    entry("DisplayName", NbtSummaryValue::string("Kills")),
                    entry("CriteriaName", NbtSummaryValue::string("killed:zombie")),
                ])]),
            )]),
        );
        assert_eq!(
            load(&paths).objectives[0].criteria,
            "minecraft.killed:minecraft.zombie"
        );
        cleanup(paths);
    }

    #[test]
    fn a_scoreboard_with_score_display_slot_and_team_round_trips() {
        let paths = scratch("full");
        let packed = Java26ScoreboardPacked {
            objectives: one_objective().objectives,
            scores: vec![Java26ScoreboardPackedScore {
                owner: "Alice".into(),
                objective: "kills".into(),
                score: 4,
                locked: false,
                display: None,
                format: None,
            }],
            display_slots: vec![(ScoreboardDisplaySlot::Sidebar, "kills".into())],
            teams: vec![Java26ScoreboardTeam {
                name: "red".into(),
                display_name: Some(NbtSummaryValue::string("Red")),
                color: Some(ChatFormatting::Red),
                allow_friendly_fire: true,
                see_friendly_invisibles: true,
                member_name_prefix: None,
                member_name_suffix: None,
                name_tag_visibility: TeamVisibility::Always,
                death_message_visibility: TeamVisibility::Always,
                collision_rule: TeamCollisionRule::Always,
                players: vec!["Alice".into()],
            }],
        };
        let document = java_26_1_scoreboard_document(&packed);
        assert_eq!(
            document,
            saved_data_document(vec![
                entry(
                    "Objectives",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("Name", NbtSummaryValue::string("kills")),
                        entry("DisplayName", NbtSummaryValue::string("Kills"),),
                    ])]),
                ),
                entry(
                    "PlayerScores",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("Name", NbtSummaryValue::string("Alice")),
                        entry("Objective", NbtSummaryValue::string("kills")),
                        entry("Score", NbtSummaryValue::Int(4)),
                    ])]),
                ),
                entry(
                    "DisplaySlots",
                    NbtSummaryValue::Compound(vec![entry(
                        "sidebar",
                        NbtSummaryValue::string("kills"),
                    )]),
                ),
                entry(
                    "Teams",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("Name", NbtSummaryValue::string("red")),
                        entry("DisplayName", NbtSummaryValue::string("Red"),),
                        entry("TeamColor", NbtSummaryValue::string("red")),
                        entry(
                            "Players",
                            NbtSummaryValue::List(vec![NbtSummaryValue::string(
                                "Alice".to_string(),
                            )]),
                        ),
                    ])]),
                ),
            ])
        );
        write(&paths, &document);
        assert_eq!(load(&paths), packed);
        let mut raw = packed;
        let name = JavaString::from(vec![0xd800]);
        let owner = JavaString::from(vec![0xdc00]);
        raw.objectives[0].name = name.clone();
        raw.scores[0].objective = name.clone();
        raw.scores[0].owner = owner.clone();
        raw.display_slots[0].1 = name.clone();
        raw.teams[0].name = name;
        raw.teams[0].players = vec![owner];
        write(&paths, &java_26_1_scoreboard_document(&raw));
        assert_eq!(load(&paths), raw);
        cleanup(paths);
    }

    #[test]
    fn canonical_objective_criteria_accepts_mod_entries_with_registry() {
        use bbb_protocol::content_registry::{
            ContentRegistryBuilder, ModBlockRegistration, ModEntityRegistration,
            ModItemRegistration,
        };

        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "modtest:ore".to_string(),
            state_count: 1,
            light_emission: 0,
            destroy_speed: 3.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_item(ModItemRegistration {
            resource_id: "modtest:gem".to_string(),
            max_stack_size: 64,
            is_block_item: false,
        });
        builder.register_entity(ModEntityRegistration {
            resource_id: "modtest:golem".to_string(),
            ..Default::default()
        });
        let registry = builder.freeze_for_session().unwrap();

        assert_eq!(
            canonical_objective_criteria_with_registry("minecraft.mined:modtest.ore", &registry),
            Some("minecraft.mined:modtest.ore".to_string())
        );
        assert_eq!(
            canonical_objective_criteria_with_registry("minecraft.used:modtest.gem", &registry),
            Some("minecraft.used:modtest.gem".to_string())
        );
        assert_eq!(
            canonical_objective_criteria_with_registry("minecraft.killed:modtest.golem", &registry),
            Some("minecraft.killed:modtest.golem".to_string())
        );
    }
}
