//! Native execution of Vanilla 26.1's `DataFixTypes.OPTIONS` program.
//!
//! Registration order, schema versions, constructor arguments, legacy LWJGL
//! key codes, and source hashes are generated from the verified Vanilla tree.
//! This module owns the decision logic of the registered `Options*Fix`
//! classes. `ClientOptions::load` invokes it on the complete raw string map
//! before any current option codec or startup binding sees a value.

use std::collections::BTreeMap;

use thiserror::Error;

use super::datafix_26_1::{CLIENT_OPTIONS_DATA_FIXES, LEGACY_LWJGL_KEY_NAMES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ClientOptionsDataFix {
    pub(super) version: i32,
    pub(super) kind: ClientOptionsDataFixKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClientOptionsDataFixKind {
    ForceVbo,
    LowerCaseLanguage,
    KeyLwjgl3,
    KeyTranslation,
    AddTextBackground,
    FancyGraphicsToMode,
    RenameField {
        source: &'static str,
        target: &'static str,
    },
    ProgrammerArt,
    AmbientOcclusion,
    AccessibilityOnboard,
    MenuBlurriness,
    GraphicsModeSplit {
        field: &'static str,
        fast: &'static str,
        fancy: &'static str,
        fabulous: &'static str,
    },
    GraphicsPresetCustom,
    MusicToast,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(super) enum ClientOptionsDataFixError {
    #[error("legacy key option {key} has non-integer LWJGL code {value:?}")]
    InvalidLwjglKeyCode { key: String, value: String },
    #[error("legacy key option {key} is too short for key translation: {value:?}")]
    KeyTranslationTooShort { key: String, value: String },
}

/// Mirrors `Options.dataFix`: a missing or invalid `version` is zero, and DFU
/// runs each options registration whose schema version is newer than the input.
pub(super) fn update_to_current_version(
    values: &mut BTreeMap<String, String>,
) -> Result<(), ClientOptionsDataFixError> {
    let version = values
        .get("version")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0);
    for fix in CLIENT_OPTIONS_DATA_FIXES {
        if version < fix.version {
            apply_fix(values, fix.kind)?;
        }
    }
    Ok(())
}

fn apply_fix(
    values: &mut BTreeMap<String, String>,
    kind: ClientOptionsDataFixKind,
) -> Result<(), ClientOptionsDataFixError> {
    match kind {
        ClientOptionsDataFixKind::ForceVbo => {
            values.insert("useVbo".to_owned(), "true".to_owned());
        }
        ClientOptionsDataFixKind::LowerCaseLanguage => {
            if let Some(language) = values.get_mut("lang") {
                *language = language.to_lowercase();
            }
        }
        ClientOptionsDataFixKind::KeyLwjgl3 => migrate_lwjgl_keys(values)?,
        ClientOptionsDataFixKind::KeyTranslation => translate_key_names(values)?,
        ClientOptionsDataFixKind::AddTextBackground => {
            if let Some(chat_opacity) = values.get("chatOpacity") {
                let opacity = parse_java_double(chat_opacity)
                    .map(|opacity| (0.9 * opacity + 0.1) / 2.0)
                    .unwrap_or(0.5);
                values.insert(
                    "textBackgroundOpacity".to_owned(),
                    bbb_protocol::java_double_to_string(opacity),
                );
            }
        }
        ClientOptionsDataFixKind::FancyGraphicsToMode => {
            rename_and_fix(values, "fancyGraphics", "graphicsMode", |value| {
                if value == "true" { "1" } else { "0" }.to_owned()
            });
        }
        ClientOptionsDataFixKind::RenameField { source, target } => {
            rename_and_fix(values, source, target, String::from);
        }
        ClientOptionsDataFixKind::ProgrammerArt => {
            for key in ["resourcePacks", "incompatibleResourcePacks"] {
                if let Some(value) = values.get_mut(key) {
                    *value = value.replace("\"programer_art\"", "\"programmer_art\"");
                }
            }
        }
        ClientOptionsDataFixKind::AmbientOcclusion => {
            if let Some(value) = values.get_mut("ao") {
                *value = match value.as_str() {
                    "0" => "false".to_owned(),
                    "1" | "2" => "true".to_owned(),
                    _ => value.clone(),
                };
            }
        }
        ClientOptionsDataFixKind::AccessibilityOnboard => {
            values.insert("onboardAccessibility".to_owned(), "false".to_owned());
        }
        ClientOptionsDataFixKind::MenuBlurriness => {
            if let Some(value) = values.get_mut("menuBackgroundBlurriness") {
                let blurriness = parse_java_float(value).unwrap_or(0.5);
                *value = java_math_round(blurriness * 10.0).to_string();
            }
        }
        ClientOptionsDataFixKind::GraphicsModeSplit {
            field,
            fast,
            fancy,
            fabulous,
        } => {
            let value = match values.get("graphicsMode").map(String::as_str) {
                Some("2") => fabulous,
                Some("0") => fast,
                _ => fancy,
            };
            values.insert(field.to_owned(), value.to_owned());
        }
        ClientOptionsDataFixKind::GraphicsPresetCustom => {
            values.insert("graphicsPreset".to_owned(), "custom".to_owned());
        }
        ClientOptionsDataFixKind::MusicToast => {
            rename_and_fix(values, "showNowPlayingToast", "musicToast", |value| {
                if value == "false" {
                    "never"
                } else {
                    "pause_and_toast"
                }
                .to_owned()
            });
        }
    }
    Ok(())
}

fn rename_and_fix(
    values: &mut BTreeMap<String, String>,
    source: &str,
    target: &str,
    update: impl FnOnce(String) -> String,
) {
    if let Some(value) = values.remove(source) {
        values.insert(target.to_owned(), update(value));
    }
}

fn migrate_lwjgl_keys(
    values: &mut BTreeMap<String, String>,
) -> Result<(), ClientOptionsDataFixError> {
    for (key, value) in values.iter_mut().filter(|(key, _)| key.starts_with("key_")) {
        let old_value =
            value
                .parse::<i32>()
                .map_err(|_| ClientOptionsDataFixError::InvalidLwjglKeyCode {
                    key: key.clone(),
                    value: value.clone(),
                })?;
        *value = if old_value < 0 {
            let button = old_value.wrapping_add(100);
            match button {
                0 => "key.mouse.left".to_owned(),
                1 => "key.mouse.right".to_owned(),
                2 => "key.mouse.middle".to_owned(),
                _ => format!("key.mouse.{}", button.wrapping_add(1)),
            }
        } else {
            LEGACY_LWJGL_KEY_NAMES
                .iter()
                .find_map(|(code, name)| (*code == old_value).then_some(*name))
                .unwrap_or("key.unknown")
                .to_owned()
        };
    }
    Ok(())
}

fn translate_key_names(
    values: &mut BTreeMap<String, String>,
) -> Result<(), ClientOptionsDataFixError> {
    for (key, value) in values.iter_mut().filter(|(key, _)| key.starts_with("key_")) {
        if value.starts_with("key.mouse") || value.starts_with("scancode.") {
            continue;
        }
        let suffix = java_substring_from_utf16(value, 4).ok_or_else(|| {
            ClientOptionsDataFixError::KeyTranslationTooShort {
                key: key.clone(),
                value: value.clone(),
            }
        })?;
        *value = format!("key.keyboard.{suffix}");
    }
    Ok(())
}

fn java_substring_from_utf16(value: &str, start: usize) -> Option<String> {
    let units: Vec<_> = value.encode_utf16().collect();
    (units.len() >= start).then(|| String::from_utf16_lossy(&units[start..]))
}

fn normalized_java_number(value: &str) -> Option<&str> {
    let value = value.trim_matches(|character| character <= '\u{20}');
    if value.is_empty() {
        return None;
    }
    if matches!(
        value,
        "NaN" | "+NaN" | "-NaN" | "Infinity" | "+Infinity" | "-Infinity"
    ) {
        return Some(value);
    }
    value
        .strip_suffix(['f', 'F', 'd', 'D'])
        .or(Some(value))
        .filter(|value| !value.is_empty())
}

fn parse_java_double(value: &str) -> Option<f64> {
    let value = normalized_java_number(value)?;
    match value {
        "NaN" | "+NaN" | "-NaN" => Some(f64::NAN),
        "Infinity" | "+Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        value if is_hex_float(value) => hexf_parse::parse_hexf64(value, false).ok(),
        value => value.parse::<f64>().ok(),
    }
}

fn parse_java_float(value: &str) -> Option<f32> {
    let value = normalized_java_number(value)?;
    match value {
        "NaN" | "+NaN" | "-NaN" => Some(f32::NAN),
        "Infinity" | "+Infinity" => Some(f32::INFINITY),
        "-Infinity" => Some(f32::NEG_INFINITY),
        value if is_hex_float(value) => hexf_parse::parse_hexf32(value, false).ok(),
        value => value.parse::<f32>().ok(),
    }
}

fn is_hex_float(value: &str) -> bool {
    value
        .strip_prefix(['+', '-'])
        .unwrap_or(value)
        .starts_with("0x")
        || value
            .strip_prefix(['+', '-'])
            .unwrap_or(value)
            .starts_with("0X")
}

/// Java `Math.round(float)`: add `0.5F`, floor, then use Java's saturating
/// float-to-int conversion (`NaN -> 0`, infinities to the respective bound).
fn java_math_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn complete_version_zero_chain_applies_in_registration_order() {
        let mut values = map(&[
            ("lang", "EN_US"),
            ("useVbo", "false"),
            ("key_key.forward", "17"),
            ("key_key.pickItem", "-98"),
            ("chatOpacity", "0.8"),
            ("textBackgroundOpacity", "0.1"),
            ("fancyGraphics", "true"),
            ("key_key.swapHands", "45"),
            ("key_key.swapOffhand", "46"),
            ("resourcePacks", "[\"programer_art\"]"),
            ("incompatibleResourcePacks", "[\"programer_art\"]"),
            ("ao", "2"),
            ("onboardAccessibility", "true"),
            ("menuBackgroundBlurriness", "0.45"),
            ("graphicsPreset", "fancy"),
            ("showNowPlayingToast", "true"),
        ]);

        update_to_current_version(&mut values).unwrap();

        assert_eq!(values["useVbo"], "true");
        assert_eq!(values["lang"], "en_us");
        assert_eq!(values["key_key.forward"], "key.keyboard.w");
        assert_eq!(values["key_key.pickItem"], "key.mouse.middle");
        assert_eq!(values["textBackgroundOpacity"], "0.41000000000000003");
        assert!(!values.contains_key("fancyGraphics"));
        assert_eq!(values["graphicsMode"], "1");
        assert!(!values.contains_key("key_key.swapHands"));
        assert_eq!(values["key_key.swapOffhand"], "key.keyboard.x");
        assert_eq!(values["resourcePacks"], "[\"programmer_art\"]");
        assert_eq!(values["incompatibleResourcePacks"], "[\"programmer_art\"]");
        assert_eq!(values["ao"], "true");
        assert_eq!(values["onboardAccessibility"], "false");
        assert_eq!(values["menuBackgroundBlurriness"], "5");
        assert_eq!(values["cutoutLeaves"], "true");
        assert_eq!(values["weatherRadius"], "10");
        assert_eq!(values["vignette"], "true");
        assert_eq!(values["improvedTransparency"], "false");
        assert_eq!(values["graphicsPreset"], "custom");
        assert!(!values.contains_key("showNowPlayingToast"));
        assert_eq!(values["musicToast"], "pause_and_toast");
    }

    #[test]
    fn missing_invalid_current_and_future_versions_match_options_data_fix() {
        for version in [None, Some("not-an-int")] {
            let mut values = map(&[("lang", "EN_US"), ("useVbo", "false")]);
            if let Some(version) = version {
                values.insert("version".to_owned(), version.to_owned());
            }
            update_to_current_version(&mut values).unwrap();
            assert_eq!(values["lang"], "en_us");
            assert_eq!(values["useVbo"], "true");
        }

        for version in [bbb_protocol::MC_DATA_VERSION, i32::MAX] {
            let mut values = map(&[
                ("version", &version.to_string()),
                ("lang", "EN_US"),
                ("useVbo", "false"),
            ]);
            update_to_current_version(&mut values).unwrap();
            assert_eq!(values["lang"], "EN_US");
            assert_eq!(values["useVbo"], "false");
        }
    }

    #[test]
    fn each_schema_boundary_is_exclusive_of_its_own_fixes() {
        let mut at_vbo = map(&[("version", "505"), ("useVbo", "false")]);
        update_to_current_version(&mut at_vbo).unwrap();
        assert_eq!(at_vbo["useVbo"], "false");

        let mut before_vbo = map(&[("version", "504"), ("useVbo", "false")]);
        update_to_current_version(&mut before_vbo).unwrap();
        assert_eq!(before_vbo["useVbo"], "true");

        let mut at_split = map(&[
            ("version", "4651"),
            ("graphicsMode", "2"),
            ("cutoutLeaves", "old"),
            ("graphicsPreset", "fabulous"),
        ]);
        update_to_current_version(&mut at_split).unwrap();
        assert_eq!(at_split["cutoutLeaves"], "old");
        assert_eq!(at_split["graphicsPreset"], "fabulous");
    }

    #[test]
    fn lwjgl_key_fix_covers_named_unknown_and_extended_mouse_branches() {
        let mut values = map(&[
            ("version", "1000"),
            ("key_named", "30"),
            ("key_unknown", "999"),
            ("key_extra_mouse", "-97"),
            ("not_a_key", "30"),
        ]);

        update_to_current_version(&mut values).unwrap();

        assert_eq!(values["key_named"], "key.keyboard.a");
        assert_eq!(values["key_unknown"], "key.keyboard.unknown");
        assert_eq!(values["key_extra_mouse"], "key.mouse.4");
        assert_eq!(values["not_a_key"], "30");
    }

    #[test]
    fn malformed_legacy_key_aborts_the_whole_options_application() {
        let mut values = map(&[("version", "0"), ("key_key.forward", "key.keyboard.w")]);

        assert_eq!(
            update_to_current_version(&mut values),
            Err(ClientOptionsDataFixError::InvalidLwjglKeyCode {
                key: "key_key.forward".to_owned(),
                value: "key.keyboard.w".to_owned(),
            })
        );

        let mut too_short = map(&[("version", "1400"), ("key_key.forward", "abc")]);
        assert_eq!(
            update_to_current_version(&mut too_short),
            Err(ClientOptionsDataFixError::KeyTranslationTooShort {
                key: "key_key.forward".to_owned(),
                value: "abc".to_owned(),
            })
        );
    }

    #[test]
    fn key_translation_keeps_mouse_and_scancode_and_uses_java_substring() {
        let mut values = map(&[
            ("version", "1400"),
            ("key_keyboard", "key.q"),
            ("key_mouse", "key.mouse.left"),
            ("key_scancode", "scancode.17"),
            ("key_nonstandard", "abcdvalue"),
        ]);

        update_to_current_version(&mut values).unwrap();

        assert_eq!(values["key_keyboard"], "key.keyboard.q");
        assert_eq!(values["key_mouse"], "key.mouse.left");
        assert_eq!(values["key_scancode"], "scancode.17");
        assert_eq!(values["key_nonstandard"], "key.keyboard.value");
    }

    #[test]
    fn text_background_and_blur_use_java_number_and_rounding_semantics() {
        let mut invalid = map(&[
            ("version", "1800"),
            ("chatOpacity", "not-a-double"),
            ("menuBackgroundBlurriness", "NaN"),
        ]);
        update_to_current_version(&mut invalid).unwrap();
        assert_eq!(invalid["textBackgroundOpacity"], "0.5");
        assert_eq!(invalid["menuBackgroundBlurriness"], "0");

        let mut java_literals = map(&[
            ("version", "1800"),
            ("chatOpacity", " Infinity "),
            ("menuBackgroundBlurriness", "0x1.0p-1F"),
        ]);
        update_to_current_version(&mut java_literals).unwrap();
        assert_eq!(java_literals["textBackgroundOpacity"], "Infinity");
        assert_eq!(java_literals["menuBackgroundBlurriness"], "5");

        assert_eq!(java_math_round(-0.5), 0);
        assert_eq!(java_math_round(f32::NAN), 0);
        assert_eq!(java_math_round(f32::INFINITY), i32::MAX);
        assert_eq!(java_math_round(f32::NEG_INFINITY), i32::MIN);
    }

    #[test]
    fn graphics_split_covers_fast_fabulous_unknown_and_missing_modes() {
        for (mode, cutout, transparency) in [
            (Some("0"), "false", "false"),
            (Some("2"), "true", "true"),
            (Some("unexpected"), "true", "false"),
            (None, "true", "false"),
        ] {
            let mut values = map(&[("version", "4600")]);
            if let Some(mode) = mode {
                values.insert("graphicsMode".to_owned(), mode.to_owned());
            }
            update_to_current_version(&mut values).unwrap();
            assert_eq!(values["cutoutLeaves"], cutout);
            assert_eq!(values["improvedTransparency"], transparency);
            assert_eq!(values["graphicsPreset"], "custom");
        }
    }

    #[test]
    fn music_toast_and_ambient_occlusion_keep_their_exact_default_branches() {
        for (old, expected) in [
            ("false", "never"),
            ("true", "pause_and_toast"),
            ("unexpected", "pause_and_toast"),
        ] {
            let mut values = map(&[("version", "4600"), ("showNowPlayingToast", old)]);
            update_to_current_version(&mut values).unwrap();
            assert_eq!(values["musicToast"], expected);
        }
        for (old, expected) in [
            ("0", "false"),
            ("1", "true"),
            ("2", "true"),
            ("other", "other"),
        ] {
            let mut values = map(&[("version", "3000"), ("ao", old)]);
            update_to_current_version(&mut values).unwrap();
            assert_eq!(values["ao"], expected);
        }
    }
}
