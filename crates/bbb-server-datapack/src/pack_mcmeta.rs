//! `pack.mcmeta` parsing for `PackType.SERVER_DATA` on the headless server.
//!
//! Vanilla anchors (Java 26.1):
//! - `PackMetadataSection.java:11-28` — the `pack` section: a required `description`
//!   (`fieldOf`) plus the format declaration parsed by
//!   `PackFormat.packCodec(SERVER_DATA)`; `FALLBACK_CODEC` keeps only the description
//!   and maps the format to `PackFormat.of(Integer.MAX_VALUE)` (undeclared).
//! - `Pack.java:43-70` (`readPackMetadata`) — the strict `pack` codec is tried first;
//!   on `JsonParseException` vanilla retries with `FALLBACK_TYPE`. A pack whose
//!   `description` is missing fails both codecs and is dropped. Errors in the
//!   `filter` / `features` / `overlays` sections are *not* covered by the fallback:
//!   they propagate to the outer catch and make the whole pack unavailable
//!   (`ResourceMetadata.fromJsonStream`, `getOrThrow`).
//! - `PackFormat.java:103-323` — `IntermediaryFormat` (`PACK_CODEC` /
//!   `OVERLAY_CODEC`) and the full `validate` / `validateNewFormat` /
//!   `validateOldFormat` / `validatePackFormatForRange` branch structure, transcribed
//!   here with `lastPreMinorVersion(SERVER_DATA) = 81` (`PackFormat.java:62-68`).
//! - `ResourceFilterSection.java` + `IdentifierPattern.java` — the `filter` section;
//!   namespace and path predicates are independent `Pattern.asPredicate()` (find)
//!   tests that default to "match everything" when the field is absent.
//! - `FeatureFlagsMetadataSection.java` — the `features` section with required
//!   `enabled` id list.
//! - `OverlayMetadataSection.java` — the `overlays` section; `entries` is required,
//!   `directory` must full-match `[-_a-zA-Z0-9.]+`, and the entry list is validated
//!   by `PackFormat.validateHolderList` (`PackFormat.java:33-60`).
//!
//! Owned deviations from vanilla (all fail-safe, documented in the slice contract):
//! - `description` is retained as the raw `RegistryJsonValue` without Component
//!   interpretation; any present JSON value is accepted.
//! - `pack_format` / `supported_formats` values below zero parse like vanilla
//!   (`Codec.INT`) but fail validation because the frozen `PackFormat` type uses
//!   `u32`; vanilla would build a negative-major format instead.

use std::collections::BTreeSet;
use std::fmt;

use bbb_registry_codec::pack_filter::{IdentifierPatternSource, ResourceFilter};
use bbb_registry_codec::{decode_loaded_registry_json, RegistryJsonValue as Value};
use regex::Regex;

use super::pack::{
    DeclaredFormatRange, PackCompatibility, PackFormat, JAVA_26_1_SERVER_DATA_FORMAT,
};

/// `PackFormat.lastPreMinorVersion(PackType.SERVER_DATA)` (`PackFormat.java:62-68`).
pub(crate) const LAST_PRE_MINOR_VERSION: i64 = 81;

/// `validatePackFormatForRange` lower bound (`PackFormat.java:319-321`).
const MIN_MULTI_VERSION_FORMAT: i64 = 15;

/// Cap on `filter.block` rule count so a hostile pack cannot multiply pattern matching cost.
const MAX_PACK_FILTER_RULES: usize = 256;

/// One unparseable or missing piece of `pack.mcmeta`; the message mirrors the
/// vanilla `DataResult.error` text where one exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackMetaError(pub String);

impl fmt::Display for PackMetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PackMetaError {}

fn err(message: impl Into<String>) -> PackMetaError {
    PackMetaError(message.into())
}

/// Parsed content of one `pack.mcmeta` for `PackType.SERVER_DATA`.
///
/// `declared_range: None` is the `PackMetadataSection.FALLBACK_CODEC` outcome: the
/// strict format declaration failed to parse and vanilla falls back to an
/// undeclared format (`UNDECLARED_FORMAT`, compatibility `Unknown`).
#[derive(Debug, Clone)]
pub struct ServerPackMeta {
    /// Raw `pack.description` JSON (`PackMetadataSection.java:13` / `:23`); kept
    /// verbatim because Component interpretation is out of scope for the headless
    /// server.
    pub description_raw: Value,
    /// Validated `[min_format, max_format]` range; `None` when undeclared.
    pub declared_range: Option<DeclaredFormatRange>,
    /// `filter` section (`ResourceFilterSection.java`); empty when absent.
    pub filter: ResourceFilter,
    /// `features.enabled` ids (`FeatureFlagsMetadataSection.java`), normalized to
    /// `namespace:path` form (`Identifier.withDefaultNamespace`).
    pub requested_features: BTreeSet<String>,
    /// `overlays.entries` (`OverlayMetadataSection.java`), in file order. Entries
    /// without any format fields are dropped with a warning in vanilla
    /// (`PackFormat.java:45-46`); applying overlays is out of scope here.
    pub overlays: Vec<OverlayDeclaration>,
}

impl ServerPackMeta {
    /// Parse one `pack.mcmeta` document, mirroring `Pack.readPackMetadata`
    /// (`Pack.java:43-70`): strict `pack` codec with `FALLBACK_CODEC` retry, and
    /// hard errors for malformed `filter` / `features` / `overlays` sections.
    pub fn parse(bytes: &[u8]) -> Result<Self, PackMetaError> {
        let source = decode_loaded_registry_json(bytes, bytes.len())
            .map_err(|e| err(format!("pack.mcmeta is not valid JSON: {e}")))?;
        let root = source.json().expect("loaded JSON retains its source");
        let pack_section = root
            .get("pack")
            .ok_or_else(|| err("missing metadata in pack: no \"pack\" section"))?;

        // `Pack.readPackMetadata`: strict codec first, FALLBACK_TYPE on failure.
        let (description_raw, declared_range) = match parse_pack_section(pack_section) {
            Ok((description, range)) => (description, Some(range)),
            Err(_) => {
                let description = pack_section.get("description").cloned().ok_or_else(|| {
                    err("missing description in \"pack\" section (fails both the strict and the fallback codec)")
                })?;
                (description, None)
            }
        };

        // Sections outside the fallback: malformed known sections fail the pack
        // (`ResourceMetadata.fromJsonStream` + `getOrThrow`, `Pack.java:66-69`).
        let filter = match root.get("filter") {
            Some(section) => parse_filter_section(section)?,
            None => ResourceFilter::default(),
        };
        let requested_features = match root.get("features") {
            Some(section) => parse_features_section(section)?,
            None => BTreeSet::new(),
        };
        let overlays = match root.get("overlays") {
            Some(section) => parse_overlays_section(section)?,
            None => Vec::new(),
        };

        Ok(Self {
            description_raw,
            declared_range,
            filter,
            requested_features,
            overlays,
        })
    }

    /// `PackCompatibility.forVersion` (`PackCompatibility.java:34-43`) against the
    /// Java 26.1 server-data format (101.1).
    pub fn compatibility(&self) -> PackCompatibility {
        match &self.declared_range {
            Some(range) => range.compatibility(JAVA_26_1_SERVER_DATA_FORMAT),
            None => PackCompatibility::Unknown,
        }
    }

    /// `OverlayMetadataSection.overlaysForVersion` (`OverlayMetadataSection.java:39-41`):
    /// declared overlay directories whose format range matches Java 26.1 (`101.1`),
    /// in file order.
    pub fn active_overlays(&self) -> Vec<&str> {
        self.overlays
            .iter()
            .filter(|entry| {
                entry
                    .range
                    .compatibility(JAVA_26_1_SERVER_DATA_FORMAT)
                    .is_compatible()
            })
            .map(|entry| entry.directory.as_str())
            .collect()
    }
}

/// One validated overlay entry (`OverlayMetadataSection.OverlayEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayDeclaration {
    /// Inclusive `[min, max]` format range in which the overlay applies.
    pub range: DeclaredFormatRange,
    /// Overlay root directory name (validated against `[-_a-zA-Z0-9.]+`,
    /// `OverlayMetadataSection.java:14`).
    pub directory: String,
}

/// `PackFormat.IntermediaryFormat` (`PackFormat.java:103`): the unvalidated
/// `min_format` / `max_format` / `pack_format` / `supported_formats` view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IntermediaryFormat {
    min: Option<PackFormat>,
    max: Option<PackFormat>,
    format: Option<i64>,
    supported: Option<(i64, i64)>,
}

impl IntermediaryFormat {
    /// `IntermediaryFormat.effectiveMinMajorVersion` (`PackFormat.java:134-140`).
    fn effective_min_major(&self) -> i64 {
        match (self.min, self.supported) {
            (Some(min), Some((supported_min, _))) => i64::from(min.major).min(supported_min),
            (Some(min), None) => i64::from(min.major),
            (None, Some((supported_min, _))) => supported_min,
            (None, None) => i64::MAX,
        }
    }

    /// `IntermediaryFormat.validate` (`PackFormat.java:142-168`).
    fn validate(
        &self,
        last_pre_minor: i64,
        has_pack_format_field: bool,
        require_old_field: bool,
        context: &str,
        old_field: &str,
    ) -> Result<DeclaredFormatRange, PackMetaError> {
        if self.min.is_some() != self.max.is_some() {
            Err(err(format!(
                "{context} missing field, must declare both min_format and max_format"
            )))
        } else if require_old_field && self.supported.is_none() {
            Err(err(format!(
                "{context} missing required field {old_field}, must be present in all overlays for any overlays to work across game versions"
            )))
        } else if self.min.is_some() {
            self.validate_new_format(
                last_pre_minor,
                has_pack_format_field,
                require_old_field,
                context,
                old_field,
            )
        } else if self.supported.is_some() {
            self.validate_old_format(last_pre_minor, has_pack_format_field, context, old_field)
        } else {
            match self.format {
                Some(main_format) if has_pack_format_field => {
                    if main_format > last_pre_minor {
                        Err(err(format!(
                            "{context} declares support for version newer than {last_pre_minor}, but is missing mandatory fields min_format and max_format"
                        )))
                    } else if main_format < 0 {
                        // bbb deviation: `PackFormat` uses `u32`, so a negative legacy
                        // `pack_format` cannot be represented; vanilla would accept it.
                        Err(err(format!("{context} declares a negative pack_format {main_format}, which is not supported")))
                    } else {
                        let format = PackFormat::new(main_format as u32, 0);
                        Ok(DeclaredFormatRange {
                            min: format,
                            max: format,
                        })
                    }
                }
                _ => Err(err(format!(
                    "{context} could not be parsed, missing format version information"
                ))),
            }
        }
    }

    /// `IntermediaryFormat.validateNewFormat` (`PackFormat.java:170-272`).
    fn validate_new_format(
        &self,
        last_pre_minor: i64,
        has_pack_format_field: bool,
        require_old_field: bool,
        context: &str,
        old_field: &str,
    ) -> Result<DeclaredFormatRange, PackMetaError> {
        let min = self.min.expect("validate dispatches on min presence");
        let max = self.max.expect("min and max are present together");
        let major_min = i64::from(min.major);
        let major_max = i64::from(max.major);
        if min > max {
            return Err(err(format!(
                "{context} min_format ({min}) is greater than max_format ({max})"
            )));
        }

        if major_min > last_pre_minor && !require_old_field {
            if self.supported.is_some() {
                return Err(err(format!(
                    "{context} key {old_field} is deprecated starting from pack format {}. Remove {old_field} from your pack.mcmeta.",
                    last_pre_minor + 1
                )));
            }
            if has_pack_format_field && self.format.is_some() {
                self.validate_pack_format_for_range(major_min, major_max)?;
            }
        } else {
            let Some((supported_min, supported_max)) = self.supported else {
                return Err(err(format!(
                    "{context} declares support for format {major_min}, but game versions supporting formats 17 to {last_pre_minor} require a {old_field} field. Add \"{old_field}\": [{major_min}, {last_pre_minor}] or require a version greater or equal to {}.0.",
                    last_pre_minor + 1
                )));
            };
            if supported_min != major_min {
                return Err(err(format!(
                    "{context} version declaration mismatch between {old_field} (from {supported_min}) and min_format ({min})"
                )));
            }
            if supported_max != major_max && supported_max != last_pre_minor {
                return Err(err(format!(
                    "{context} version declaration mismatch between {old_field} (up to {supported_max}) and max_format ({max})"
                )));
            }
            if has_pack_format_field {
                if self.format.is_none() {
                    return Err(err(format!(
                        "{context} declares support for formats up to {last_pre_minor}, but game versions supporting formats 17 to {last_pre_minor} require a pack_format field. Add \"pack_format\": {major_min} or require a version greater or equal to {}.0.",
                        last_pre_minor + 1
                    )));
                }
                self.validate_pack_format_for_range(major_min, major_max)?;
            }
        }

        Ok(DeclaredFormatRange { min, max })
    }

    /// `IntermediaryFormat.validateOldFormat` (`PackFormat.java:274-312`).
    fn validate_old_format(
        &self,
        last_pre_minor: i64,
        has_pack_format_field: bool,
        context: &str,
        old_field: &str,
    ) -> Result<DeclaredFormatRange, PackMetaError> {
        let (supported_min, supported_max) = self
            .supported
            .expect("validate dispatches on supported presence");
        if supported_max > last_pre_minor {
            return Err(err(format!(
                "{context} declares support for version newer than {last_pre_minor}, but is missing mandatory fields min_format and max_format"
            )));
        }
        if has_pack_format_field {
            if self.format.is_none() {
                return Err(err(format!(
                    "{context} declares support for formats up to {last_pre_minor}, but game versions supporting formats 17 to {last_pre_minor} require a pack_format field. Add \"pack_format\": {supported_min} or require a version greater or equal to {}.0.",
                    last_pre_minor + 1
                )));
            }
            self.validate_pack_format_for_range(supported_min, supported_max)?;
        }
        if supported_min < 0 {
            // bbb deviation: `PackFormat` uses `u32`; see `validate`.
            return Err(err(format!("{context} declares a negative {old_field} minimum {supported_min}, which is not supported")));
        }
        Ok(DeclaredFormatRange {
            min: PackFormat::new(supported_min as u32, 0),
            max: PackFormat::new(supported_max as u32, 0),
        })
    }

    /// `IntermediaryFormat.validatePackFormatForRange` (`PackFormat.java:314-323`).
    fn validate_pack_format_for_range(&self, min: i64, max: i64) -> Result<(), PackMetaError> {
        let main_format = self.format.expect("caller checks presence");
        if main_format < min || main_format > max {
            Err(err(format!("Pack declared support for versions {min} to {max} but declared main format is {main_format}")))
        } else if main_format < MIN_MULTI_VERSION_FORMAT {
            Err(err(
                "Multi-version packs cannot support minimum version of less than 15, since this will leave versions in range unable to load pack.",
            ))
        } else {
            Ok(())
        }
    }
}

/// Strict parse of the `pack` section (`PackMetadataSection.java:20-28` with
/// `PackFormat.packCodec(SERVER_DATA)`, `PackFormat.java:70-77`).
fn parse_pack_section(section: &Value) -> Result<(Value, DeclaredFormatRange), PackMetaError> {
    let description = section
        .get("description")
        .cloned()
        .ok_or_else(|| err("missing required field \"description\" in \"pack\" section"))?;
    let intermediary = parse_intermediary(section, "supported_formats", true)?;
    let range = intermediary.validate(
        LAST_PRE_MINOR_VERSION,
        true,
        false,
        "Pack",
        "supported_formats",
    )?;
    Ok((description, range))
}

/// Parse the shared `IntermediaryFormat` fields. With `has_pack_format_field` this
/// is `PACK_CODEC` (`PackFormat.java:104-112`, legacy field `supported_formats`);
/// otherwise it is `OVERLAY_CODEC` (`PackFormat.java:113-120`, legacy field
/// `formats`, with `format` derived from `min` and unused by validation).
fn parse_intermediary(
    section: &Value,
    old_field: &str,
    has_pack_format_field: bool,
) -> Result<IntermediaryFormat, PackMetaError> {
    let min = section
        .get("min_format")
        .map(|v| parse_pack_format(v, 0))
        .transpose()?;
    let max = section
        .get("max_format")
        .map(|v| parse_pack_format(v, u32::MAX))
        .transpose()?;
    let format = if has_pack_format_field {
        section.get("pack_format").map(parse_i32).transpose()?
    } else {
        // `OVERLAY_CODEC` sets `format = min.map(PackFormat::major)`; validation
        // never reads it when `hasPackFormatField` is false, so it stays `None`.
        None
    };
    let supported = section
        .get(old_field)
        .map(|v| parse_int_inclusive_range(v, old_field))
        .transpose()?;
    Ok(IntermediaryFormat {
        min,
        max,
        format,
        supported,
    })
}

/// `ExtraCodecs.compactListCodec(NON_NEGATIVE_INT, listOf(1, 256))` +
/// `fullCodec(defaultMinor)` (`PackFormat.java:22-31`): either a bare non-negative
/// int (minor defaults) or a 1..=256 element list of non-negative ints whose first
/// two elements are `(major, minor)`.
fn parse_pack_format(value: &Value, default_minor: u32) -> Result<PackFormat, PackMetaError> {
    match value {
        Value::Array(items) => {
            if items.is_empty() || items.len() > 256 {
                return Err(err(format!(
                    "pack format list must have between 1 and 256 elements, got {}",
                    items.len()
                )));
            }
            let mut elements = Vec::with_capacity(items.len());
            for item in items {
                elements.push(parse_non_negative_int(item)?);
            }
            let major = elements[0];
            let minor = if elements.len() > 1 {
                elements[1]
            } else {
                i64::from(default_minor)
            };
            // `default_minor` is `0` or `u32::MAX`; both fit the `i64` used above.
            Ok(PackFormat::new(major as u32, minor as u32))
        }
        single => Ok(PackFormat::new(
            parse_non_negative_int(single)? as u32,
            default_minor,
        )),
    }
}

/// `ExtraCodecs.NON_NEGATIVE_INT` (`ExtraCodecs.java:148`): an integer in
/// `0..=i32::MAX`.
fn parse_non_negative_int(value: &Value) -> Result<i64, PackMetaError> {
    let parsed = parse_i32(value)?;
    if parsed < 0 {
        return Err(err(format!("Value must be non-negative: {parsed}")));
    }
    Ok(parsed)
}

/// `Codec.INT`: a JSON integer in `i32` range (rejects floats like vanilla).
fn parse_i32(value: &Value) -> Result<i64, PackMetaError> {
    let number = value
        .as_i64()
        .ok_or_else(|| err(format!("expected an integer, got {value:?}")))?;
    if number < i64::from(i32::MIN) || number > i64::from(i32::MAX) {
        return Err(err(format!("integer {number} is out of 32-bit range")));
    }
    Ok(number)
}

/// `InclusiveRange.codec(Codec.INT)` (`InclusiveRange.java:26-29` +
/// `ExtraCodecs.intervalCodec`, `ExtraCodecs.java:271-296`): a single int meaning
/// `[v, v]`, or exactly `[min, max]` with `min <= max`.
fn parse_int_inclusive_range(value: &Value, field: &str) -> Result<(i64, i64), PackMetaError> {
    let (min, max) = match value {
        Value::Array(items) => {
            if items.len() != 2 {
                return Err(err(format!(
                    "\"{field}\" list must have exactly 2 elements, got {}",
                    items.len()
                )));
            }
            (parse_i32(&items[0])?, parse_i32(&items[1])?)
        }
        single => {
            let parsed = parse_i32(single)?;
            (parsed, parsed)
        }
    };
    if min > max {
        return Err(err(
            "min_inclusive must be less than or equal to max_inclusive",
        ));
    }
    Ok((min, max))
}

/// `ResourceFilterSection` codec (`ResourceFilterSection.java:10-13`): required
/// `block` list of `IdentifierPattern` entries.
fn parse_filter_section(section: &Value) -> Result<ResourceFilter, PackMetaError> {
    let block = section
        .get("block")
        .ok_or_else(|| err("missing required field \"block\" in \"filter\" section"))?
        .as_array()
        .ok_or_else(|| err("\"filter.block\" is not a JSON array"))?;
    if block.len() > MAX_PACK_FILTER_RULES {
        return Err(err(format!(
            "\"filter.block\" has {} rules, exceeding the {MAX_PACK_FILTER_RULES} rule limit",
            block.len()
        )));
    }
    let mut patterns = Vec::with_capacity(block.len());
    for entry in block {
        entry
            .as_object()
            .ok_or_else(|| err("\"filter.block\" entry is not a JSON object"))?;
        let source = IdentifierPatternSource {
            namespace: pattern_field(entry, "namespace")?,
            path: pattern_field(entry, "path")?,
        };
        patterns.push(source.compile().map_err(PackMetaError)?);
    }
    Ok(ResourceFilter::new(patterns))
}

fn pattern_field(entry: &Value, field: &str) -> Result<Option<String>, PackMetaError> {
    entry
        .get(field)
        .map(|value| match value {
            Value::String(pattern) => pattern
                .try_to_string()
                .map_err(|error| err(error.to_string())),
            _ => Err(err(format!("\"filter.block.{field}\" is not a string"))),
        })
        .transpose()
}

/// `FeatureFlagsMetadataSection` codec (`FeatureFlagsMetadataSection.java:10-13`):
/// required `enabled` list of identifiers. Vanilla additionally rejects unknown
/// flag ids with a partial result (`FeatureFlagRegistry.codec`); the headless
/// server retains the ids verbatim (normalized to `namespace:path`) instead.
fn parse_features_section(section: &Value) -> Result<BTreeSet<String>, PackMetaError> {
    let enabled = section
        .get("enabled")
        .ok_or_else(|| err("missing required field \"enabled\" in \"features\" section"))?
        .as_array()
        .ok_or_else(|| err("\"features.enabled\" is not a JSON array"))?;
    let mut flags = BTreeSet::new();
    for flag in enabled {
        let flag = match flag {
            Value::String(flag) => flag.with_utf8(normalize_identifier),
            _ => None,
        }
        .ok_or_else(|| err("\"features.enabled\" entry is not a string"))??;
        flags.insert(flag);
    }
    Ok(flags)
}

/// `Identifier` validation (`Identifier.java`): `[a-z0-9_.-]` namespace and
/// `[a-z0-9_./-]` path, default namespace `minecraft`.
fn normalize_identifier(raw: &str) -> Result<String, PackMetaError> {
    let (namespace, path) = match raw.split_once(':') {
        Some((namespace, path)) => (namespace, path),
        None => ("minecraft", raw),
    };
    let valid_namespace = !namespace.is_empty()
        && namespace
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'));
    let valid_path = !path.is_empty()
        && path.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '/' | '-')
        });
    if !valid_namespace || !valid_path {
        return Err(err(format!("invalid identifier \"{raw}\"")));
    }
    Ok(format!("{namespace}:{path}"))
}

/// `OverlayMetadataSection` codec (`OverlayMetadataSection.java:25-29` +
/// `OverlayEntry.listCodecForPackType`, `:44-64`): required `entries` list,
/// validated as a group by `PackFormat.validateHolderList` (`PackFormat.java:33-60`).
fn parse_overlays_section(section: &Value) -> Result<Vec<OverlayDeclaration>, PackMetaError> {
    /// `OverlayMetadataSection.DIR_VALIDATOR` (`OverlayMetadataSection.java:14`),
    /// anchored because Java uses `Matcher.matches()` (full match).
    const DIR_VALIDATOR: &str = r"^[-_a-zA-Z0-9.]+$";

    let directory_validator = Regex::new(DIR_VALIDATOR).expect("constant pattern compiles");
    let entries = section
        .get("entries")
        .ok_or_else(|| err("missing required field \"entries\" in \"overlays\" section"))?
        .as_array()
        .ok_or_else(|| err("\"overlays.entries\" is not a JSON array"))?;

    let mut intermediates = Vec::with_capacity(entries.len());
    for entry in entries {
        let format = parse_intermediary(entry, "formats", false)?;
        let directory = entry
            .get("directory")
            .and_then(Value::utf8_string)
            .ok_or_else(|| {
                err("missing required field \"directory\" in \"overlays.entries\" entry")
            })?;
        if !directory_validator.is_match(&directory) {
            return Err(err(format!("{directory} is not accepted directory name")));
        }
        intermediates.push((format, directory));
    }

    // `PackFormat.validateHolderList` (`PackFormat.java:33-60`): the legacy
    // `formats` field becomes mandatory for every entry as soon as any entry still
    // reaches a pre-minor version.
    let min_version = intermediates
        .iter()
        .map(|(format, _)| format.effective_min_major())
        .min()
        .unwrap_or(i64::MAX);
    let require_old_field = min_version <= LAST_PRE_MINOR_VERSION;

    let mut overlays = Vec::with_capacity(intermediates.len());
    for (format, directory) in intermediates {
        if format.min.is_none() && format.max.is_none() && format.supported.is_none() {
            // Vanilla logs "Unknown or broken overlay entry" and skips it
            // (`PackFormat.java:45-46`).
            continue;
        }
        let range = format.validate(
            LAST_PRE_MINOR_VERSION,
            false,
            require_old_field,
            &format!("Overlay \"{directory}\""),
            "formats",
        )?;
        overlays.push(OverlayDeclaration { range, directory });
    }
    Ok(overlays)
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value as JsonValue};

    use super::*;

    fn format(major: u32, minor: u32) -> PackFormat {
        PackFormat::new(major, minor)
    }

    fn intermediary(
        min: Option<PackFormat>,
        max: Option<PackFormat>,
        format: Option<i64>,
        supported: Option<(i64, i64)>,
    ) -> IntermediaryFormat {
        IntermediaryFormat {
            min,
            max,
            format,
            supported,
        }
    }

    /// `validate` with the `Pack` context used by `PACK_CODEC`
    /// (`PackFormat.java:74`): hasPackFormatField=true, requireOldField=false.
    fn validate_pack(f: &IntermediaryFormat) -> Result<DeclaredFormatRange, PackMetaError> {
        f.validate(
            LAST_PRE_MINOR_VERSION,
            true,
            false,
            "Pack",
            "supported_formats",
        )
    }

    fn parse_mcmeta(mcmeta: JsonValue) -> Result<ServerPackMeta, PackMetaError> {
        ServerPackMeta::parse(mcmeta.to_string().as_bytes())
    }

    // --- IntermediaryFormat.validate branch coverage (PackFormat.java:142-323) ---

    #[test]
    fn validate_accepts_new_format_above_pre_minor() {
        let range = validate_pack(&intermediary(
            Some(format(102, 0)),
            Some(format(102, 3)),
            None,
            None,
        ))
        .unwrap();
        assert_eq!(
            range,
            DeclaredFormatRange {
                min: format(102, 0),
                max: format(102, 3)
            }
        );
    }

    #[test]
    fn validate_old_format_supported_only() {
        let range = validate_pack(&intermediary(None, None, Some(80), Some((80, 81)))).unwrap();
        assert_eq!(
            range,
            DeclaredFormatRange {
                min: format(80, 0),
                max: format(81, 0)
            }
        );
    }

    #[test]
    fn validate_legacy_pack_format_only() {
        // pack_format <= 81 alone declares the single-version range.
        let range = validate_pack(&intermediary(None, None, Some(81), None)).unwrap();
        assert_eq!(
            range,
            DeclaredFormatRange {
                min: format(81, 0),
                max: format(81, 0)
            }
        );
        // pack_format > 81 alone is an error: min/max are mandatory there.
        let e = validate_pack(&intermediary(None, None, Some(102), None)).unwrap_err();
        assert!(
            e.0.contains("missing mandatory fields min_format and max_format"),
            "{e}"
        );
    }

    // --- pack.mcmeta document parsing ---

    #[test]
    fn parses_modern_pack_section() {
        let meta = parse_mcmeta(json!({
            "pack": {
                "description": {"text": "modern pack"},
                "min_format": [101, 0],
                "max_format": [101, 1]
            }
        }))
        .unwrap();
        assert_eq!(
            meta.description_raw
                .get("text")
                .unwrap()
                .utf8_string()
                .as_deref(),
            Some("modern pack")
        );
        assert_eq!(
            meta.declared_range,
            Some(DeclaredFormatRange {
                min: format(101, 0),
                max: format(101, 1)
            })
        );
        assert_eq!(meta.compatibility(), PackCompatibility::Compatible);
    }

    #[test]
    fn bare_int_min_max_format_default_minors() {
        // BOTTOM_CODEC defaults minor to 0, TOP_CODEC to `u32::MAX`
        // (`PackFormat.java:22-31`).
        let meta = parse_mcmeta(json!({
            "pack": {"description": "d", "min_format": 100, "max_format": 101}
        }))
        .unwrap();
        assert_eq!(
            meta.declared_range,
            Some(DeclaredFormatRange {
                min: format(100, 0),
                max: format(101, u32::MAX)
            })
        );
        assert_eq!(meta.compatibility(), PackCompatibility::Compatible);
    }

    #[test]
    fn compatibility_classification() {
        let cases = [
            (
                json!({"pack": {"description": "d", "pack_format": 81}}),
                PackCompatibility::TooOld,
            ),
            (
                json!({"pack": {"description": "d", "min_format": [102, 0], "max_format": [102, 0]}}),
                PackCompatibility::TooNew,
            ),
            (
                json!({"pack": {"description": "d", "min_format": [100, 0], "max_format": [200, 0]}}),
                PackCompatibility::Compatible,
            ),
        ];
        for (mcmeta, expected) in cases {
            assert_eq!(parse_mcmeta(mcmeta).unwrap().compatibility(), expected);
        }
    }

    #[test]
    fn strict_pack_section_failure_falls_back_to_undeclared() {
        // `pack_format: 102` alone fails the strict codec; `Pack.readPackMetadata`
        // retries with FALLBACK_TYPE and the pack loads as undeclared/Unknown.
        let meta = parse_mcmeta(json!({"pack": {"description": "d", "pack_format": 102}})).unwrap();
        assert_eq!(meta.declared_range, None);
        assert_eq!(meta.compatibility(), PackCompatibility::Unknown);
        // Malformed known fields (wrong JSON type) take the same fallback path.
        let meta = parse_mcmeta(
            json!({"pack": {"description": "d", "min_format": "abc", "max_format": 101}}),
        )
        .unwrap();
        assert_eq!(meta.declared_range, None);
    }

    #[test]
    fn unknown_extra_fields_are_tolerated() {
        let meta = parse_mcmeta(json!({
            "pack": {"description": "d", "pack_format": 81, "custom": {"any": 1}},
            "unknown_section": [1, 2, 3]
        }))
        .unwrap();
        assert_eq!(
            meta.declared_range,
            Some(DeclaredFormatRange {
                min: format(81, 0),
                max: format(81, 0)
            })
        );
    }

    // --- filter section (ResourceFilterSection / IdentifierPattern) ---

    #[test]
    fn parses_filter_section_and_blocks_paths() {
        let meta = parse_mcmeta(json!({
            "pack": {"description": "d", "pack_format": 81},
            "filter": {"block": [
                {"namespace": "(?<=mine)craft", "path": r"\Qrecipes/\E(?=foo)"},
                {"path": "^secret$"}
            ]}
        }))
        .unwrap();
        // Namespace + path pattern: find semantics (`Pattern.asPredicate`).
        assert!(meta.filter.is_path_blocked("minecraft", "recipes/foo"));
        // The second entry's namespace predicate defaults to true.
        assert!(meta.filter.is_path_blocked("other", "secret"));
        assert!(!meta.filter.is_path_blocked("other", "public"));
    }

    #[test]
    fn filter_namespace_only_entry_blocks_everything_like_vanilla() {
        // `IdentifierPattern` predicates default to true, so an entry with only a
        // `namespace` pattern has an always-true path predicate. The filter still
        // joins only namespaces matching the namespace predicate
        // (`MultiPackResourceManager.java:38-49`), so every lower-priority path is
        // blocked in `minecraft` and nothing is blocked in other namespaces.
        let meta = parse_mcmeta(json!({
            "pack": {"description": "d", "pack_format": 81},
            "filter": {"block": [{"namespace": "minecraft"}]}
        }))
        .unwrap();
        assert!(meta.filter.is_namespace_filtered("minecraft"));
        assert!(!meta.filter.is_namespace_filtered("other"));
        assert!(meta.filter.is_path_blocked("minecraft", "anything"));
        assert!(!meta.filter.is_path_blocked("other", "anything"));
    }

    #[test]
    fn filter_section_rejects_excessive_rule_counts() {
        let within_limit: Vec<JsonValue> = (0..MAX_PACK_FILTER_RULES)
            .map(|index| json!({"namespace": "minecraft", "path": format!("rule_{index}")}))
            .collect();
        parse_mcmeta(json!({
            "pack": {"description": "d", "min_format": 101, "max_format": 101},
            "filter": {"block": within_limit}
        }))
        .unwrap();

        let over_limit: Vec<JsonValue> = (0..=MAX_PACK_FILTER_RULES)
            .map(|index| json!({"namespace": "minecraft", "path": format!("rule_{index}")}))
            .collect();
        let error = parse_mcmeta(json!({
            "pack": {"description": "d", "min_format": 101, "max_format": 101},
            "filter": {"block": over_limit}
        }))
        .unwrap_err();
        assert!(error.0.contains("rule limit"), "{error}");
    }

    // --- features section (FeatureFlagsMetadataSection) ---

    #[test]
    fn parses_features_section_with_default_namespace() {
        let meta = parse_mcmeta(json!({
            "pack": {"description": "d", "pack_format": 81},
            "features": {"enabled": ["trade_rebalance", "minecraft:redstone_experiments"]}
        }))
        .unwrap();
        assert_eq!(
            meta.requested_features,
            BTreeSet::from([
                "minecraft:trade_rebalance".to_string(),
                "minecraft:redstone_experiments".to_string()
            ])
        );
    }

    // --- overlays section (OverlayMetadataSection / validateHolderList) ---

    #[test]
    fn parses_overlay_entries() {
        let meta = parse_mcmeta(json!({
            "pack": {"description": "d", "min_format": 101, "max_format": 101},
            "overlays": {"entries": [
                {"formats": [80, 81], "directory": "legacy_overlay"},
                {"min_format": [102, 0], "max_format": [102, 0], "formats": [102, 102], "directory": "future"}
            ]}
        }))
        .unwrap();
        assert_eq!(
            meta.overlays,
            vec![
                OverlayDeclaration {
                    range: DeclaredFormatRange {
                        min: format(80, 0),
                        max: format(81, 0)
                    },
                    directory: "legacy_overlay".to_string()
                },
                OverlayDeclaration {
                    range: DeclaredFormatRange {
                        min: format(102, 0),
                        max: format(102, 0)
                    },
                    directory: "future".to_string()
                },
            ]
        );
    }

    #[test]
    fn overlay_entry_without_format_fields_is_skipped() {
        // `PackFormat.java:45-46`: vanilla warns and drops the entry.
        let meta = parse_mcmeta(json!({
            "pack": {"description": "d", "min_format": 101, "max_format": 101},
            "overlays": {"entries": [
                {"directory": "broken"},
                {"min_format": [102, 0], "max_format": [102, 0], "directory": "ok"}
            ]}
        }))
        .unwrap();
        assert_eq!(meta.overlays.len(), 1);
        assert_eq!(meta.overlays[0].directory, "ok");
    }
}
