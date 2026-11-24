//! The pack directory `/datapack create` writes under the world's `datapacks`
//! folder.
//!
//! Vanilla anchors (Java 26.1):
//! - `DataPackCommand.createPack` (`DataPackCommand.java:165-211`) — the whole
//!   refusal ladder and the two directories plus `pack.mcmeta` it writes.
//! - `FileUtil.java:20-21`, `:83-85`, `:153-159` — the two id predicates.
//! - `PackFormat.java:22-31`, `:87-89`, `:104-112`, `:122-132` — how the
//!   current version's `minorRange` reaches `pack.mcmeta`.
//! - `GsonHelper.writeValue` (`GsonHelper.java:504-538`) through a `JsonWriter`
//!   with `setSerializeNulls(false)` and `setIndent("  ")`.

use std::fs;
use std::path::Path;

use bbb_protocol::{
    canonical_component_codec_value, java_double_to_string, java_float_to_string,
    nbt_summary::with_last_wins_compound_entries, CanonicalComponentCodecValue,
    CanonicalComponentOwnerValue, JavaString, NbtSummaryValue,
};
use bbb_registry_codec::RegistryJsonValue as JsonValue;
use regex::Regex;

use crate::pack::{PackFormat, JAVA_26_1_SERVER_DATA_FORMAT};
use crate::pack_mcmeta::LAST_PRE_MINOR_VERSION;

/// `PackType.SERVER_DATA.getDirectory()` (`PackType.java:5`, `:13-15`).
const SERVER_DATA_DIRECTORY: &str = "data";

/// `PackMetadataSection.SERVER_TYPE.name()` (`PackMetadataSection.java:17`).
const PACK_SECTION: &str = "pack";

// `IntermediaryFormat.fromRange` (`PackFormat.java:122-132`) writes
// `pack_format` and `supported_formats` beside the two range fields when the
// major range holds `lastPreMinorVersion`; `minorRange` makes that range one
// major, so a game format above 81 never carries the deprecated pair.
const _: () = assert!(JAVA_26_1_SERVER_DATA_FORMAT.major as i64 > LAST_PRE_MINOR_VERSION);

/// One refusal `createPack` throws, in the order it tests for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatePackRefusal {
    /// `ERROR_PACK_INVALID_NAME` (`DataPackCommand.java:64-66`, `:167-169`).
    InvalidName,
    /// `ERROR_PACK_INVALID_FULL_NAME` (`DataPackCommand.java:67-69`, `:171-173`).
    InvalidFullName,
    /// `ERROR_PACK_ALREADY_EXISTS` (`DataPackCommand.java:70-72`, `:176-178`).
    AlreadyExists,
    /// `ERROR_PACK_METADATA_ENCODE_FAILURE` (`DataPackCommand.java:73-75`,
    /// `:184-187`) carrying the codec message it names.
    MetadataEncodeFailure(String),
    /// `ERROR_PACK_IO_FAILURE` (`DataPackCommand.java:76-78`, `:204-207`).
    IoFailure,
}

/// `DataPackCommand.createPack` (`DataPackCommand.java:165-211`) below the
/// command layer.
///
/// `datapacks_dir` is `None` for a world that keeps no save directory at all,
/// which has nowhere to write; vanilla always resolves one off the save
/// (`LevelResource.DATAPACK_DIR`) and answers a missing one from
/// `Files.createDirectory` as an I/O failure.
pub fn create_pack(
    datapacks_dir: Option<&Path>,
    id: &str,
    description: &NbtSummaryValue,
) -> Result<(), CreatePackRefusal> {
    if !is_valid_path_segment(id) {
        return Err(CreatePackRefusal::InvalidName);
    }
    if !is_path_part_portable(id) {
        return Err(CreatePackRefusal::InvalidFullName);
    }
    let Some(datapacks_dir) = datapacks_dir else {
        return Err(CreatePackRefusal::IoFailure);
    };
    let pack_dir = datapacks_dir.join(id);
    if pack_dir.exists() {
        return Err(CreatePackRefusal::AlreadyExists);
    }
    let mcmeta =
        pack_mcmeta_document(description).map_err(CreatePackRefusal::MetadataEncodeFailure)?;
    write_pack_directory(&pack_dir, &mcmeta).map_err(|_| CreatePackRefusal::IoFailure)
}

/// `DataPackCommand.java:192-203`: two `createDirectory` calls, neither of
/// which builds a missing parent, and then the metadata file.
fn write_pack_directory(pack_dir: &Path, mcmeta: &str) -> std::io::Result<()> {
    fs::create_dir(pack_dir)?;
    fs::create_dir(pack_dir.join(SERVER_DATA_DIRECTORY))?;
    fs::write(pack_dir.join("pack.mcmeta"), mcmeta)
}

/// `FileUtil.STRICT_PATH_SEGMENT_CHECK` (`FileUtil.java:21`) as
/// `containsAllowedCharactersOnly` (`:153-155`) applies it.
fn contains_allowed_characters_only(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|byte| matches!(byte, b'-' | b'.' | b'_' | b'a'..=b'z' | b'0'..=b'9'))
}

/// `FileUtil.isValidPathSegment` (`FileUtil.java:157-159`).
pub fn is_valid_path_segment(segment: &str) -> bool {
    segment != ".." && segment != "." && contains_allowed_characters_only(segment)
}

/// `FileUtil.isPathPartPortable` (`FileUtil.java:83-85`): the name must not
/// full-match `RESERVED_WINDOWS_FILENAMES` (`FileUtil.java:20`), which is
/// case-insensitive and whose `.` never crosses a line terminator.
pub fn is_path_part_portable(name: &str) -> bool {
    let reserved =
        Regex::new(r"(?i)\A(?:.*\.|(?:COM|CLOCK\$|CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\..*)?)\z")
            .expect("the reserved-filename pattern is a constant");
    !reserved.is_match(name)
}

/// The `pack.mcmeta` document `createPack` writes (`DataPackCommand.java:180-203`).
fn pack_mcmeta_document(description: &NbtSummaryValue) -> Result<String, String> {
    let description = component_json(canonical_component_codec_value(description)?)?;
    let mut section = vec![("description".into(), description)];
    section.extend(format_fields(JAVA_26_1_SERVER_DATA_FORMAT));
    serde_json::to_string_pretty(&JsonValue::Object(vec![(
        PACK_SECTION.into(),
        JsonValue::Object(section),
    )]))
    .map_err(|error| error.to_string())
}

/// `PackFormat.IntermediaryFormat.PACK_CODEC` (`PackFormat.java:104-112`) over
/// `minorRange` (`PackFormat.java:87-89`): the minimum keeps its minor through
/// `BOTTOM_CODEC` and the open maximum collapses to a bare major through
/// `TOP_CODEC` (`PackFormat.java:22-31`).
fn format_fields(game: PackFormat) -> Vec<(JavaString, JsonValue)> {
    vec![
        ("min_format".into(), compact_format(game, 0)),
        (
            "max_format".into(),
            compact_format(PackFormat::new(game.major, u32::MAX), u32::MAX),
        ),
    ]
}

/// `ExtraCodecs.compactListCodec` (`ExtraCodecs.java:415-418`): a one-element
/// list is written as the element itself.
fn compact_format(format: PackFormat, default_minor: u32) -> JsonValue {
    if format.minor == default_minor {
        JsonValue::Number(format.major.to_string())
    } else {
        JsonValue::Array(vec![
            JsonValue::Number(format.major.to_string()),
            JsonValue::Number(format.minor.to_string()),
        ])
    }
}

/// `ComponentSerialization.CODEC.encodeStart(JsonOps.INSTANCE, ..)`
/// (`DataPackCommand.java:183`) over the canonical value the component codec
/// already produced.
/// `KeyDispatchCodec.encode` adds the dispatched map before the type field, so
/// the `action` a click or hover event dispatches on (`ClickEvent.java:20`,
/// `HoverEvent.java:20`) is the last member written rather than the first. Only
/// codec-named fields are reordered; an `action` inside a custom click payload
/// is data and keeps its place.
fn component_json(value: CanonicalComponentCodecValue<'_>) -> Result<JsonValue, String> {
    Ok(match value {
        CanonicalComponentCodecValue::Boolean(value) => JsonValue::Bool(value),
        CanonicalComponentCodecValue::Int(value) => JsonValue::Number(value.to_string()),
        CanonicalComponentCodecValue::String(value) => JsonValue::String(value.into_owned()),
        CanonicalComponentCodecValue::List(values) => JsonValue::Array(
            values
                .into_iter()
                .map(component_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        CanonicalComponentCodecValue::IntArray(values) => number_array(&values),
        CanonicalComponentCodecValue::Object(fields) => {
            let mut members = Vec::new();
            let mut action = None;
            for (name, value) in fields {
                let value = component_json(value)?;
                if name == "action" {
                    action = Some(value);
                } else {
                    members.push((name.into(), value));
                }
            }
            members.extend(action.map(|value| ("action".into(), value)));
            JsonValue::Object(members)
        }
        CanonicalComponentCodecValue::Nbt(value) => raw_nbt_json(value)?,
        CanonicalComponentCodecValue::Owner(kind, _) => {
            return Err(format!(
                "Can't encode {} without its owning codec",
                owner_name(kind)
            ))
        }
    })
}

fn raw_nbt_json(value: &NbtSummaryValue) -> Result<JsonValue, String> {
    Ok(match value {
        NbtSummaryValue::Byte(value) => JsonValue::Number(value.to_string()),
        NbtSummaryValue::Short(value) => JsonValue::Number(value.to_string()),
        NbtSummaryValue::Int(value) => JsonValue::Number(value.to_string()),
        NbtSummaryValue::Long(value) => JsonValue::Number(value.to_string()),
        NbtSummaryValue::Float(bits) => {
            JsonValue::Number(finite_number(java_float_to_string(f32::from_bits(*bits)))?)
        }
        NbtSummaryValue::Double(bits) => {
            JsonValue::Number(finite_number(java_double_to_string(f64::from_bits(*bits)))?)
        }
        NbtSummaryValue::ByteArray(values) => number_array(values),
        NbtSummaryValue::String(value) => JsonValue::String(value.clone()),
        NbtSummaryValue::List(values) => JsonValue::Array(
            values
                .iter()
                .map(raw_nbt_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        NbtSummaryValue::Compound(entries) => {
            JsonValue::Object(with_last_wins_compound_entries(entries, |entries| {
                entries
                    .iter()
                    .map(|entry| Ok((entry.name.clone(), raw_nbt_json(&entry.value)?)))
                    .collect::<Result<Vec<_>, String>>()
            })?)
        }
        NbtSummaryValue::IntArray(values) => number_array(values),
        NbtSummaryValue::LongArray(values) => number_array(values),
    })
}

fn number_array<T: ToString>(values: &[T]) -> JsonValue {
    JsonValue::Array(
        values
            .iter()
            .map(|value| JsonValue::Number(value.to_string()))
            .collect(),
    )
}

/// `JsonWriter.value(Number)` refuses a non-finite value outside lenient mode,
/// which escapes `createPack` as an unchecked throw rather than as a refusal.
fn finite_number(encoded: String) -> Result<String, String> {
    match encoded.as_str() {
        "NaN" | "Infinity" | "-Infinity" => {
            Err(format!("Numeric values must be finite, but was {encoded}"))
        }
        _ => Ok(encoded),
    }
}

fn owner_name(kind: CanonicalComponentOwnerValue) -> &'static str {
    match kind {
        CanonicalComponentOwnerValue::PlayerProfile => "a player profile",
        CanonicalComponentOwnerValue::Dialog => "a dialog reference",
        CanonicalComponentOwnerValue::ItemComponentPatch => "an item component patch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bbb_protocol::{parse_command_snbt_value_prefix, NbtSummaryEntry};

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "bbb-client-pack-create-test-{}-{tag}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// The description travels as the SNBT `ComponentArgument` read
    /// (`ComponentArgument.java:29`), so the fixtures are spelled that way.
    fn description(snbt: &str) -> NbtSummaryValue {
        let (consumed, value) = parse_command_snbt_value_prefix(snbt).expect(snbt);
        assert_eq!(consumed, snbt.len(), "{snbt} is one SNBT value");
        value
    }

    /// The constant tail every document carries beside its description.
    fn document(description_json: &str) -> String {
        format!(
            "{{\n  \"pack\": {{\n    \"description\": {description_json},\n    \"min_format\": [\n      101,\n      1\n    ],\n    \"max_format\": 101\n  }}\n}}"
        )
    }

    fn written(snbt: &str) -> String {
        pack_mcmeta_document(&description(snbt)).expect(snbt)
    }

    #[test]
    fn a_dispatched_action_is_the_last_member_of_its_event() {
        for (snbt, expected) in [
            (
                r#"{"text":"x","hover_event":{"action":"show_text","value":"y"}}"#,
                "{\n      \"text\": \"x\",\n      \"hover_event\": {\n        \"value\": \"y\",\n        \"action\": \"show_text\"\n      }\n    }",
            ),
            (
                r#"{"text":"x","hover_event":{"action":"show_item","id":"minecraft:stone","count":2}}"#,
                "{\n      \"text\": \"x\",\n      \"hover_event\": {\n        \"id\": \"minecraft:stone\",\n        \"count\": 2,\n        \"action\": \"show_item\"\n      }\n    }",
            ),
            (
                r#"{"text":"x","hover_event":{"action":"show_entity","id":"minecraft:pig","uuid":"12345678-1234-1234-1234-123456789012","name":"Piggy"}}"#,
                "{\n      \"text\": \"x\",\n      \"hover_event\": {\n        \"id\": \"minecraft:pig\",\n        \"uuid\": [\n          305419896,\n          305402420,\n          305402420,\n          1450741778\n        ],\n        \"name\": \"Piggy\",\n        \"action\": \"show_entity\"\n      }\n    }",
            ),
            (
                r#"{"text":"x","click_event":{"action":"open_url","url":"https://example.com/"}}"#,
                "{\n      \"text\": \"x\",\n      \"click_event\": {\n        \"url\": \"https://example.com/\",\n        \"action\": \"open_url\"\n      }\n    }",
            ),
            // The `action` inside the custom payload is data, so it stays where
            // the NBT put it while the event's own moves last.
            (
                r#"{"text":"x","click_event":{"action":"custom","id":"minecraft:foo","payload":{"action":1}}}"#,
                "{\n      \"text\": \"x\",\n      \"click_event\": {\n        \"id\": \"minecraft:foo\",\n        \"payload\": {\n          \"action\": 1\n        },\n        \"action\": \"custom\"\n      }\n    }",
            ),
        ] {
            assert_eq!(written(snbt), document(expected), "{snbt}");
        }

        let entry = |name: JavaString, value| NbtSummaryEntry { name, value };
        let custom_description = |payload| {
            NbtSummaryValue::Compound(vec![
                entry("text".into(), NbtSummaryValue::string("x")),
                entry(
                    "click_event".into(),
                    NbtSummaryValue::Compound(vec![
                        entry("action".into(), NbtSummaryValue::string("custom")),
                        entry("id".into(), NbtSummaryValue::string("minecraft:foo")),
                        entry("payload".into(), payload),
                    ]),
                ),
            ])
        };
        let raw = custom_description(NbtSummaryValue::Compound(vec![
            entry("action".into(), NbtSummaryValue::Byte(1)),
            entry(vec![0xd800].into(), NbtSummaryValue::Short(2)),
            entry(vec![0xd801].into(), NbtSummaryValue::Long(3)),
            entry("float".into(), NbtSummaryValue::Float(0.1_f32.to_bits())),
            entry(
                "double".into(),
                NbtSummaryValue::Double(f64::from(0.1_f32).to_bits()),
            ),
            entry("bytes".into(), NbtSummaryValue::ByteArray(vec![-1, 2])),
            entry("ints".into(), NbtSummaryValue::IntArray(vec![3])),
            entry("longs".into(), NbtSummaryValue::LongArray(vec![i64::MAX])),
            entry(
                vec![0xd800].into(),
                NbtSummaryValue::string_from_utf16(vec![0xdfff]),
            ),
        ]));
        assert_eq!(
            serde_json::to_string(
                &component_json(canonical_component_codec_value(&raw).unwrap()).unwrap()
            )
            .unwrap(),
            r#"{"text":"x","click_event":{"id":"minecraft:foo","payload":{"action":1,"\ud800":"\udfff","\ud801":3,"float":0.1,"double":0.10000000149011612,"bytes":[-1,2],"ints":[3],"longs":[9223372036854775807]},"action":"custom"}}"#,
        );
        for payload in [
            NbtSummaryValue::Float(f32::NAN.to_bits()),
            NbtSummaryValue::Float(f32::INFINITY.to_bits()),
            NbtSummaryValue::Float(f32::NEG_INFINITY.to_bits()),
            NbtSummaryValue::Double(f64::NAN.to_bits()),
            NbtSummaryValue::Double(f64::INFINITY.to_bits()),
            NbtSummaryValue::Double(f64::NEG_INFINITY.to_bits()),
        ] {
            assert!(pack_mcmeta_document(&custom_description(payload)).is_err());
        }
    }

    #[test]
    fn the_two_name_predicates_answer_what_fileutil_answers() {
        for name in ["pack", "my-pack", "my_pack", "a.b", "0"] {
            assert!(is_valid_path_segment(name), "{name}");
            assert!(is_path_part_portable(name), "{name}");
        }
        for name in ["", ".", "..", "Pack", "my pack", "a/b", "pä"] {
            assert!(!is_valid_path_segment(name), "{name}");
        }
        for name in ["con", "nul", "com1", "lpt9", "aux.txt", "pack."] {
            assert!(!is_path_part_portable(name), "{name}");
        }
    }

    #[test]
    fn a_written_pack_is_read_back_as_a_compatible_one() {
        let world = TempDir::new("round-trip");
        let datapacks = world.0.join("datapacks");
        fs::create_dir(&datapacks).unwrap();
        create_pack(Some(&datapacks), "pack", &description(r#""My pack""#)).unwrap();
        let meta = crate::pack_mcmeta::ServerPackMeta::parse(
            &fs::read(datapacks.join("pack/pack.mcmeta")).unwrap(),
        )
        .unwrap();
        assert!(meta.compatibility().is_compatible());
    }
}
