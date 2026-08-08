//! Vanilla-compatible `options.txt` persistence.
//!
//! `Options.load` reads the file line by line, splitting each on its first
//! `:` only and skipping anything that does not split; `Options.save` truncates
//! the file and writes `version:` followed by every key in
//! `processDumpedOptions` then `processOptions` order. Neither side is atomic
//! and neither keeps a backup — that is `ServerList`, not `Options` — and a key
//! Vanilla no longer knows is simply not written back.
//!
//! The keys, their on-disk encodings and their defaults are generated from
//! `Options.java` into [`catalog_26_1`], so adding a Vanilla option cannot
//! silently skip this owner.

pub(crate) mod args;
mod catalog_26_1;
mod datafix;
mod datafix_26_1;
pub(crate) mod derived;
pub(crate) mod key_bindings;
mod key_mappings_26_1;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub(crate) use catalog_26_1::{
    ClientOptionDefault, ClientOptionEncoding, ClientOptionSpec, CLIENT_OPTIONS,
    CLIENT_OPTION_PREFIXES, CLIENT_OPTION_REPORT_EXTRA_KEYS,
};

/// Facts about the running host that Vanilla resolves through the JDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ClientOptionHost {
    /// `Locale.getDefault().getLanguage().equalsIgnoreCase("ja")`.
    pub(crate) locale_is_japanese: bool,
    /// `Util.getPlatform() == Util.OS.WINDOWS`.
    pub(crate) platform_is_windows: bool,
}

/// `Locale.getDefault().getLanguage().equalsIgnoreCase("ja")`. `sys-locale`
/// returns a BCP-47/POSIX-shaped locale on the supported desktop platforms;
/// only the leading language subtag participates in Vanilla's default.
pub(crate) fn locale_is_japanese(locale: Option<&str>) -> bool {
    locale
        .and_then(|locale| locale.split(['-', '_']).next())
        .is_some_and(|language| language.eq_ignore_ascii_case("ja"))
}

/// The client's `options.txt` state: every catalogued key, plus the per-element
/// lines the three trailing loops own, plus the one special startup line
/// (`fullscreenResolution`) that `Options.save` writes outside the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientOptions {
    values: BTreeMap<&'static str, String>,
    /// Lines under `key_`, `soundCategory_` and `modelPart_`, carried through
    /// verbatim after any version-required data fix, so a shared game directory
    /// keeps the current bytes Vanilla wrote. The `key_` half is read back
    /// through `key_bindings`. The special
    /// `fullscreenResolution` startup line is deliberately not here: it is
    /// stored in [`Self::fullscreen_resolution`] so the per-element groups
    /// never accidentally persist report-only fields.
    dynamic: BTreeMap<String, String>,
    /// `Options.fullscreenVideoModeString` (`Options.java:1676`): the one
    /// `fullscreenResolution` line read right after `processOptions` and
    /// written after the catalog/prefix groups only when present
    /// (`Options.java:1759-1761`). `None` is the absent preferred mode.
    fullscreen_resolution: Option<String>,
}

/// The one `options.txt` line `Options.save` writes outside the generated
/// catalog (`Options.java:1759-1761`): the stored `fullscreenResolution`
/// startup video mode. `Options.load` reads it specially too
/// (`Options.java:1676`). The other `dumpOptionsForReport` fields are report
/// fields only, never preserved as raw options lines.
pub(crate) const FULLSCREEN_RESOLUTION_KEY: &str = "fullscreenResolution";

impl ClientOptions {
    /// The state Vanilla starts from when `options.txt` does not exist.
    pub(crate) fn defaults(host: ClientOptionHost) -> Self {
        Self {
            values: CLIENT_OPTIONS
                .iter()
                .map(|spec| (spec.key, default_text(spec, host).to_owned()))
                .collect(),
            dynamic: BTreeMap::new(),
            fullscreen_resolution: None,
        }
    }

    /// `Options.load`: a missing file leaves the defaults in place, and a line
    /// that does not split on `:` is skipped rather than failing the load.
    pub(crate) fn load(path: &Path, host: ClientOptionHost) -> Result<Self> {
        let mut options = Self::defaults(host);
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(options),
            Err(error) => return Err(error.into()),
        };
        let mut raw_options = BTreeMap::new();
        for line in raw.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            raw_options.insert(key.to_owned(), value.to_owned());
        }
        if let Err(error) = datafix::update_to_current_version(&mut raw_options) {
            tracing::warn!(%error, "failed to migrate options.txt; retaining defaults");
            return Ok(options);
        }
        for (key, value) in raw_options {
            options.set_raw(&key, &value);
        }
        Ok(options)
    }

    /// `Options.save`: truncate, write the data version, then every catalogued
    /// key in `save` order followed by the per-element groups. Unknown keys are
    /// not carried over, matching Vanilla. The write is a plain truncate, also
    /// matching Vanilla — `Options.save` opens a `FileOutputStream` directly and
    /// only logs on failure, unlike `ServerList.save`, which is the client's one
    /// `Util.safeReplaceFile` caller.
    pub(crate) fn save(&self, path: &Path) -> Result<()> {
        fs::write(path, self.rendered())?;
        Ok(())
    }

    pub(crate) fn rendered(&self) -> String {
        let mut out = format!("version:{}\n", bbb_protocol::MC_DATA_VERSION);
        for spec in CLIENT_OPTIONS.iter() {
            if let Some(value) = self.values.get(spec.key) {
                out.push_str(spec.key);
                out.push(':');
                out.push_str(value);
                out.push('\n');
            }
        }
        for prefix in CLIENT_OPTION_PREFIXES {
            for (key, value) in &self.dynamic {
                if key.starts_with(prefix) {
                    out.push_str(key);
                    out.push(':');
                    out.push_str(value);
                    out.push('\n');
                }
            }
        }
        // `Options.save` (Options.java:1759-1761) writes the special
        // `fullscreenResolution` startup line after the per-element groups,
        // and only while a preferred mode exists. No other report field is
        // preserved as a raw line.
        if let Some(value) = &self.fullscreen_resolution {
            out.push_str(FULLSCREEN_RESOLUTION_KEY);
            out.push(':');
            out.push_str(value);
            out.push('\n');
        }
        out
    }

    /// `Options.dumpOptionsForReport`: the `processDumpedOptions` block plus
    /// the seven report fields the dump names itself (including
    /// `fullscreenResolution`), sorted by key. `fullscreenResolution` is
    /// always present exactly once: the stored string when a mode exists and
    /// the literal `"null"` when it does not, matching
    /// `String.valueOf(fullscreenVideoModeString)`. A crash report carries
    /// this so a bad frame can be read against the settings that produced it;
    /// these fields are report-only and are never a generic persistence
    /// whitelist.
    pub(crate) fn dump_for_report(&self) -> Vec<(String, String)> {
        let mut entries: Vec<(String, String)> = CLIENT_OPTIONS
            .iter()
            .filter(|spec| spec.dumped)
            .chain(
                CLIENT_OPTION_REPORT_EXTRA_KEYS
                    .iter()
                    .filter_map(|key| client_option_spec(key)),
            )
            .filter_map(|spec| {
                self.raw(spec.key)
                    .map(|value| (spec.key.to_owned(), value.to_owned()))
            })
            .chain(std::iter::once((
                FULLSCREEN_RESOLUTION_KEY.to_owned(),
                self.fullscreen_resolution
                    .as_deref()
                    .unwrap_or("null")
                    .to_owned(),
            )))
            .collect();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        entries.dedup_by(|left, right| left.0 == right.0);
        entries
    }

    /// The text currently on the line for `key`, if the catalog has one or the
    /// key is the special `fullscreenResolution` startup line.
    pub(crate) fn raw(&self, key: &str) -> Option<&str> {
        if key == FULLSCREEN_RESOLUTION_KEY {
            return self.fullscreen_resolution.as_deref();
        }
        self.values.get(key).map(String::as_str)
    }

    pub(crate) fn dynamic_raw(&self, key: &str) -> Option<&str> {
        self.dynamic.get(key).map(String::as_str)
    }

    #[cfg(test)]
    pub(crate) fn set_dynamic_raw_for_test(&mut self, key: &str, value: &str) {
        self.dynamic.insert(key.to_owned(), value.to_owned());
    }

    #[cfg(test)]
    pub(crate) fn set_raw_for_test(&mut self, key: &str, value: &str) {
        self.set_raw(key, value);
    }

    fn set_raw(&mut self, key: &str, value: &str) {
        if let Some(spec) = CLIENT_OPTIONS.iter().find(|spec| spec.key == key) {
            self.values.insert(spec.key, value.to_owned());
            return;
        }
        // `Options.load` (Options.java:1676) treats the special startup
        // `fullscreenResolution` line separately from the catalog, in its own
        // field rather than the per-element groups; anything else unknown is
        // never consumed and dropped on the next save.
        if key == FULLSCREEN_RESOLUTION_KEY {
            self.fullscreen_resolution = Some(value.to_owned());
            return;
        }
        if CLIENT_OPTION_PREFIXES
            .iter()
            .any(|prefix| key.starts_with(prefix))
        {
            self.dynamic.insert(key.to_owned(), value.to_owned());
        }
    }

    /// A boolean line, for the keys whose encoding is a bare `true`/`false`.
    #[cfg(test)]
    pub(crate) fn boolean(&self, key: &str) -> Option<bool> {
        match self.raw(key)? {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    /// An integer line. `Options` logs and keeps the current value when the
    /// text does not parse, so a bad line reads as absent here.
    #[cfg(test)]
    pub(crate) fn integer(&self, key: &str) -> Option<i32> {
        self.raw(key)?.parse().ok()
    }

    #[cfg(test)]
    pub(crate) fn double(&self, key: &str) -> Option<f64> {
        self.raw(key)?.parse().ok()
    }

    /// A JSON-quoted line (`mainHand:"right"`), unwrapped to its text.
    #[cfg(test)]
    pub(crate) fn quoted(&self, key: &str) -> Option<&str> {
        let raw = self.raw(key)?;
        raw.strip_prefix('"')?.strip_suffix('"')
    }
}

/// The running session's `options.txt`, kept so a toggle can write it back.
///
/// Vanilla's writers outside the options screen are the hotkeys in
/// `KeyboardHandler`: each flips one field and calls `Options.save`, which
/// rewrites the whole file. BBB implements two of them —
/// `keyDebugShowAdvancedTooltips` (F3+H) and `keyDebugFocusPause` (F3+P) — so a
/// toggle survives a restart the way it does in the official client.
pub(crate) struct ClientOptionsFile {
    path: PathBuf,
    options: ClientOptions,
}

impl ClientOptionsFile {
    pub(crate) fn load(path: PathBuf, host: ClientOptionHost) -> Result<Self> {
        let options = ClientOptions::load(&path, host)?;
        Ok(Self { path, options })
    }

    pub(crate) fn options(&self) -> &ClientOptions {
        &self.options
    }

    /// Keep the in-memory `Options` lists synchronized with the startup
    /// repository decision. Vanilla does not save here; a later hotkey save
    /// rewrites the already-reconciled lists along with every other option.
    pub(crate) fn apply_resource_pack_reconciliation(
        &mut self,
        reconciliation: &bbb_pack::ClientPackOptionsReconciliation,
    ) {
        let resource_packs = serde_json::to_string(&reconciliation.resource_packs)
            .expect("a list of strings is always valid JSON");
        let incompatible = serde_json::to_string(&reconciliation.incompatible_resource_packs)
            .expect("a list of strings is always valid JSON");
        self.options.set_raw("resourcePacks", &resource_packs);
        self.options
            .set_raw("incompatibleResourcePacks", &incompatible);
    }

    /// Apply the lines a hotkey changed, then rewrite the file. Nothing is
    /// written when every value already reads that way, because Vanilla only
    /// reaches `save` from a branch that actually flipped the field.
    pub(crate) fn apply(&mut self, writes: &[ClientOptionWrite]) -> Result<bool> {
        let mut changed = false;
        let mut enum_text = String::new();
        for write in writes {
            let spec = client_option_spec(write.key)
                .unwrap_or_else(|| panic!("{} is not a catalogued option key", write.key));
            let text = match write.value {
                ClientOptionValue::Boolean(value) => {
                    assert_eq!(
                        spec.encoding,
                        ClientOptionEncoding::Boolean,
                        "{} is not written as a boolean",
                        write.key
                    );
                    bool_text(value)
                }
                ClientOptionValue::EnumId(value) => {
                    assert_eq!(
                        spec.encoding,
                        ClientOptionEncoding::EnumId,
                        "{} is not written as an enum id",
                        write.key
                    );
                    enum_text.clear();
                    enum_text.push_str(&value.to_string());
                    enum_text.as_str()
                }
            };
            if self.options.raw(spec.key) != Some(text) {
                self.options.set_raw(spec.key, text);
                changed = true;
            }
        }
        if !changed {
            return Ok(false);
        }
        self.options
            .save(&self.path)
            .with_context(|| format!("write {}", self.path.display()))?;
        Ok(true)
    }
}

/// One `options.txt` line a hotkey toggled, carried from the input owner to the
/// thread that may touch the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientOptionWrite {
    pub(crate) key: &'static str,
    pub(crate) value: ClientOptionValue,
}

/// The two encodings a hotkey can write today: `Options.save` writes booleans
/// as `true`/`false` and enum options as their numeric id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientOptionValue {
    Boolean(bool),
    EnumId(i32),
}

/// The catalogued spec for `key`, so a binding can be checked against the
/// encoding `save` actually writes.
pub(crate) fn client_option_spec(key: &str) -> Option<&'static ClientOptionSpec> {
    CLIENT_OPTIONS.iter().find(|spec| spec.key == key)
}

fn default_text(spec: &ClientOptionSpec, host: ClientOptionHost) -> &'static str {
    match spec.default {
        ClientOptionDefault::Written(text) => text,
        ClientOptionDefault::LocaleIsJapanese => bool_text(host.locale_is_japanese),
        ClientOptionDefault::PlatformIsWindows => bool_text(host.platform_is_windows),
    }
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn unique_options_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "bbb-client-options-{name}-{}-{nanos}.txt",
            process::id()
        ))
    }

    fn options() -> ClientOptions {
        ClientOptions::defaults(ClientOptionHost::default())
    }

    #[test]
    fn the_fullscreen_resolution_line_is_retained_across_load_and_save() {
        let path = unique_options_path("fullscreen-resolution");
        fs::write(&path, "fullscreenResolution:1920x1080@144:24\n").unwrap();

        let reloaded = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();
        assert_eq!(
            reloaded.raw("fullscreenResolution"),
            Some("1920x1080@144:24")
        );
        // The special startup line has dedicated storage: it is never visible
        // through the per-element groups `dynamic_raw` reads.
        assert_eq!(reloaded.dynamic_raw("fullscreenResolution"), None);

        reloaded.save(&path).unwrap();
        let again = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();
        assert_eq!(again.raw("fullscreenResolution"), Some("1920x1080@144:24"));
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("fullscreenResolution:1920x1080@144:24"));
        assert_eq!(again.dynamic_raw("fullscreenResolution"), None);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn an_absent_fullscreen_resolution_is_omitted_from_save_and_reported_as_null() {
        let options = options();

        // `Options.save` writes the special line only while a preferred mode
        // exists; the defaults have none, so the rendered text omits it.
        assert_eq!(options.raw("fullscreenResolution"), None);
        assert!(!options.rendered().contains("fullscreenResolution"));

        // `dumpOptionsForReport` always names it as
        // `String.valueOf(fullscreenVideoModeString)`, which is the literal
        // `"null"` when the field is absent.
        let report = options.dump_for_report();
        assert_eq!(
            report
                .iter()
                .filter(|(key, _)| key == "fullscreenResolution")
                .count(),
            1
        );
        assert_eq!(
            report
                .iter()
                .find(|(key, _)| key == "fullscreenResolution")
                .map(|(_, value)| value.as_str()),
            Some("null")
        );

        // Loading a stored line routes it into the dedicated field: the save
        // round-trips it and the report switches from `null` to the stored
        // string, still exactly once.
        let path = unique_options_path("fullscreen-resolution-absent");
        fs::write(&path, "fullscreenResolution:1920x1080@144:24\n").unwrap();
        let loaded = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();
        loaded.save(&path).unwrap();
        assert_eq!(
            fs::read_to_string(&path)
                .unwrap()
                .matches("fullscreenResolution")
                .count(),
            1
        );
        assert_eq!(
            loaded
                .dump_for_report()
                .iter()
                .filter(|(key, _)| key == "fullscreenResolution")
                .count(),
            1
        );
        assert_eq!(
            loaded
                .dump_for_report()
                .iter()
                .find(|(key, _)| key == "fullscreenResolution")
                .map(|(_, value)| value.as_str()),
            Some("1920x1080@144:24")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn unknown_or_report_only_raw_lines_are_dropped_while_fullscreen_resolution_survives() {
        // `Options.save` preserves only the special `fullscreenResolution`
        // startup line plus the catalog: an unknown line is never rewritten,
        // and a report-only field must not be re-emitted from a generic
        // report-extra whitelist. `glDebugVerbosity` is a catalogued option,
        // so it is written exactly once as its own catalog line rather than a
        // second time from the report block.
        let path = unique_options_path("report-only-extra");
        fs::write(
            &path,
            "fullscreenResolution:1920x1080@144:24\n\
             glDebugVerbosity:1\n\
             futureUnknownOption:true\n",
        )
        .unwrap();

        let reloaded = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();
        reloaded.save(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();

        assert!(text.contains("fullscreenResolution:1920x1080@144:24"));
        assert_eq!(text.matches("glDebugVerbosity").count(), 1);
        assert!(!text.contains("futureUnknownOption"));

        // The report block emits each catalogued report field exactly once
        // (never twice from a generic whitelist) and `fullscreenResolution`
        // exactly once with the stored value.
        let report = reloaded.dump_for_report();
        for key in [
            "glDebugVerbosity",
            "overrideHeight",
            "overrideWidth",
            "syncChunkWrites",
            "useNativeTransport",
            "resourcePacks",
        ] {
            assert_eq!(
                report.iter().filter(|(k, _)| k == key).count(),
                1,
                "{key} must be emitted exactly once"
            );
        }
        assert_eq!(
            report
                .iter()
                .filter(|(key, _)| key == "fullscreenResolution")
                .count(),
            1
        );
        assert_eq!(
            report
                .iter()
                .find(|(key, _)| key == "fullscreenResolution")
                .map(|(_, value)| value.as_str()),
            Some("1920x1080@144:24")
        );
    }

    #[test]
    fn a_debug_hotkey_toggle_rewrites_the_file_and_survives_a_reload() {
        let path = unique_options_path("hotkey");
        fs::write(&path, "advancedItemTooltips:false\npauseOnLostFocus:true\n").unwrap();
        let mut file = ClientOptionsFile::load(path.clone(), ClientOptionHost::default()).unwrap();

        assert!(file
            .apply(&[
                ClientOptionWrite {
                    key: "advancedItemTooltips",
                    value: ClientOptionValue::Boolean(true),
                },
                ClientOptionWrite {
                    key: "pauseOnLostFocus",
                    value: ClientOptionValue::Boolean(false),
                },
            ])
            .unwrap());

        let reloaded = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();
        assert_eq!(reloaded.raw("advancedItemTooltips"), Some("true"));
        assert_eq!(reloaded.raw("pauseOnLostFocus"), Some("false"));
        // `save` rewrites the whole file, so the untouched keys come back too.
        assert_eq!(reloaded.raw("renderDistance"), Some("12"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn first_server_tutorial_sentinel_rewrites_and_survives_reload() {
        let path = unique_options_path("joined-first-server");
        let mut file = ClientOptionsFile::load(path.clone(), ClientOptionHost::default()).unwrap();

        assert!(file
            .apply(&[ClientOptionWrite {
                key: "joinedFirstServer",
                value: ClientOptionValue::Boolean(true),
            }])
            .unwrap());

        let reloaded = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();
        assert_eq!(reloaded.raw("joinedFirstServer"), Some("true"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn an_enum_option_write_stores_the_numeric_id() {
        let path = unique_options_path("enum-id");
        let mut file = ClientOptionsFile::load(path.clone(), ClientOptionHost::default()).unwrap();

        assert!(file
            .apply(&[ClientOptionWrite {
                key: "narrator",
                value: ClientOptionValue::EnumId(3),
            }])
            .unwrap());

        let reloaded = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();
        assert_eq!(reloaded.raw("narrator"), Some("3"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn a_toggle_back_to_the_stored_value_writes_nothing() {
        let path = unique_options_path("noop");
        let mut file = ClientOptionsFile::load(path.clone(), ClientOptionHost::default()).unwrap();

        // The file does not exist, so the defaults are in place; writing the
        // default back is not a change and must not create the file, the same
        // way `KeyboardHandler` only reaches `save` from a branch that flipped
        // the field.
        assert!(!file
            .apply(&[ClientOptionWrite {
                key: "pauseOnLostFocus",
                value: ClientOptionValue::Boolean(true),
            }])
            .unwrap());
        assert!(!path.exists());

        assert!(file
            .apply(&[ClientOptionWrite {
                key: "pauseOnLostFocus",
                value: ClientOptionValue::Boolean(false),
            }])
            .unwrap());
        assert!(path.exists());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn defaults_carry_the_text_vanilla_writes_for_every_catalogued_key() {
        let options = options();

        assert_eq!(options.raw("ao"), Some("true"));
        assert_eq!(options.raw("renderDistance"), Some("12"));
        // The scalar overloads write their value bare; only the OptionInstance
        // overload goes through GSON and keeps its quotes.
        assert_eq!(options.raw("lang"), Some("en_us"));
        assert_eq!(options.raw("mainHand"), Some("\"right\""));
        assert_eq!(options.quoted("mainHand"), Some("right"));
        // `fov` stores 70 but its codec writes `(70 - 70.0) / 40.0`.
        assert_eq!(options.raw("fov"), Some("0.0"));
        assert_eq!(options.raw("narrator"), Some("0"));
        assert_eq!(options.raw("resourcePacks"), Some("[]"));
    }

    #[test]
    fn host_derived_defaults_follow_the_host() {
        let japanese = ClientOptions::defaults(ClientOptionHost {
            locale_is_japanese: true,
            platform_is_windows: true,
        });

        assert_eq!(japanese.raw("japaneseGlyphVariants"), Some("true"));
        assert_eq!(japanese.raw("syncChunkWrites"), Some("true"));
        assert_eq!(options().raw("japaneseGlyphVariants"), Some("false"));
        assert_eq!(options().raw("syncChunkWrites"), Some("false"));
    }

    #[test]
    fn japanese_font_default_uses_only_the_system_language_subtag() {
        assert!(locale_is_japanese(Some("ja")));
        assert!(locale_is_japanese(Some("ja-JP")));
        assert!(locale_is_japanese(Some("JA_jp.UTF-8")));
        assert!(!locale_is_japanese(Some("en-JP")));
        assert!(!locale_is_japanese(None));
    }

    #[test]
    fn a_line_splits_on_its_first_colon_only_and_bad_lines_are_skipped() {
        let path = unique_options_path("splitting");
        fs::write(
            &path,
            "version:4786\n\
             lang:zh_cn\n\
             this line has no separator\n\
             lastServer:example.com:25565\n\
             renderDistance:16\n",
        )
        .expect("write options");

        let options = ClientOptions::load(&path, ClientOptionHost::default()).expect("load");

        assert_eq!(options.raw("lang"), Some("zh_cn"));
        // `Splitter.on(':').limit(2)` keeps the port in the value.
        assert_eq!(options.raw("lastServer"), Some("example.com:25565"));
        assert_eq!(options.integer("renderDistance"), Some(16));
        // The unsplittable line neither fails the load nor lands anywhere.
        assert_eq!(options.raw("this line has no separator"), None);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn old_options_are_data_fixed_before_current_fields_are_decoded() {
        let path = unique_options_path("datafix-before-decode");
        fs::write(
            &path,
            "version:4650\n\
             graphicsMode:2\n\
             cutoutLeaves:false\n\
             weatherRadius:1\n\
             vignette:false\n\
             improvedTransparency:false\n\
             graphicsPreset:fabulous\n\
             showNowPlayingToast:false\n",
        )
        .unwrap();

        let options = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();

        // All five v4651 fixes overwrite the already-present current fields
        // from the old graphicsMode before `set_raw` reaches their codecs.
        assert_eq!(options.raw("cutoutLeaves"), Some("true"));
        assert_eq!(options.raw("weatherRadius"), Some("10"));
        assert_eq!(options.raw("vignette"), Some("true"));
        assert_eq!(options.raw("improvedTransparency"), Some("true"));
        assert_eq!(options.raw("graphicsPreset"), Some("custom"));
        // The next schema registration renames and converts the old boolean.
        assert_eq!(options.raw("musicToast"), Some("never"));
        assert_eq!(options.raw("showNowPlayingToast"), None);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn a_throwing_legacy_key_fix_retains_all_defaults_like_options_load() {
        let path = unique_options_path("datafix-failure-defaults");
        fs::write(
            &path,
            "version:0\n\
             lang:zh_cn\n\
             renderDistance:20\n\
             key_key.forward:key.keyboard.w\n",
        )
        .unwrap();

        let options = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();

        // Options.load catches the unchecked OptionsKeyLwjgl3Fix parse error
        // outside the entire migration/decode block, so no earlier raw line is
        // applied either.
        assert_eq!(
            options,
            ClientOptions::defaults(ClientOptionHost::default())
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn a_missing_file_leaves_the_defaults_in_place() {
        let options =
            ClientOptions::load(&unique_options_path("absent"), ClientOptionHost::default())
                .expect("load");

        assert_eq!(
            options,
            ClientOptions::defaults(ClientOptionHost::default())
        );
    }

    #[test]
    fn save_round_trips_and_drops_keys_this_client_does_not_know() {
        let path = unique_options_path("round-trip");
        fs::write(
            &path,
            format!(
                "version:{}\n\
                 lang:zh_cn\n\
                 someModOption:7\n\
                 key_key.attack:key.mouse.left\n\
                 soundCategory_master:0.5\n",
                bbb_protocol::MC_DATA_VERSION
            ),
        )
        .expect("write options");

        let loaded = ClientOptions::load(&path, ClientOptionHost::default()).expect("load");
        loaded.save(&path).expect("save");
        let written = fs::read_to_string(&path).expect("read back");

        assert!(written.starts_with(&format!("version:{}\n", bbb_protocol::MC_DATA_VERSION)));
        assert!(written.contains("\nlang:zh_cn\n"));
        // Vanilla never consumes an unknown key, so `save` does not write it.
        assert!(!written.contains("someModOption"));
        // The per-element groups survive; BBB has no KeyMapping catalog to
        // rewrite them from, so dropping them would destroy real bindings.
        assert!(written.contains("\nkey_key.attack:key.mouse.left\n"));
        assert!(written.contains("\nsoundCategory_master:0.5\n"));

        let reloaded = ClientOptions::load(&path, ClientOptionHost::default()).expect("reload");
        assert_eq!(reloaded, loaded);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn a_later_hotkey_save_uses_the_reconciled_resource_pack_lists() {
        let path = unique_options_path("resource-pack-reconciliation");
        fs::write(
            &path,
            "resourcePacks:[\"missing\",\"selected\"]\n\
             incompatibleResourcePacks:[\"selected\",\"stale\"]\n\
             advancedItemTooltips:false\n",
        )
        .unwrap();
        let root = path.with_extension("packs");
        let repository = root.join("resourcepacks");
        let selected = repository.join("selected");
        fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        fs::create_dir_all(&selected).unwrap();
        fs::write(
            selected.join("pack.mcmeta"),
            r#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
        )
        .unwrap();
        let roots = bbb_pack::PackRoots::from_root(&root).unwrap();
        let reconciliation = bbb_pack::SelectedClientPackStack::from_options_repository(
            &roots,
            &repository,
            &["missing".to_owned(), "selected".to_owned()],
            &["selected".to_owned(), "stale".to_owned()],
        )
        .unwrap();
        let mut file = ClientOptionsFile::load(path.clone(), ClientOptionHost::default()).unwrap();

        file.apply_resource_pack_reconciliation(&reconciliation);
        file.apply(&[ClientOptionWrite {
            key: "advancedItemTooltips",
            value: ClientOptionValue::Boolean(true),
        }])
        .unwrap();

        let reloaded = ClientOptions::load(&path, ClientOptionHost::default()).unwrap();
        assert_eq!(reloaded.raw("resourcePacks"), Some(r#"["selected"]"#));
        assert_eq!(
            reloaded.raw("incompatibleResourcePacks"),
            Some(r#"["stale"]"#)
        );
        let _ = fs::remove_file(path);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn save_writes_every_catalogued_key_in_vanilla_order() {
        let rendered = options().rendered();
        let keys: Vec<&str> = rendered
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once(':').map(|(key, _)| key))
            .collect();

        assert_eq!(keys.len(), CLIENT_OPTIONS.len());
        let expected: Vec<&str> = CLIENT_OPTIONS.iter().map(|spec| spec.key).collect();
        assert_eq!(keys, expected);
        // `processDumpedOptions` runs first, so `ao` opens the file.
        assert_eq!(keys.first(), Some(&"ao"));
    }

    #[test]
    fn every_encoding_parses_its_own_default() {
        for spec in CLIENT_OPTIONS.iter() {
            let options = options();
            let key = spec.key;
            match spec.encoding {
                ClientOptionEncoding::Boolean => {
                    assert!(options.boolean(key).is_some(), "{key}");
                }
                ClientOptionEncoding::Integer | ClientOptionEncoding::EnumId => {
                    assert!(options.integer(key).is_some(), "{key}");
                }
                ClientOptionEncoding::Double => {
                    assert!(options.double(key).is_some(), "{key}");
                }
                ClientOptionEncoding::EnumName
                | ClientOptionEncoding::EnumNameOrBoolean
                | ClientOptionEncoding::Text => {
                    assert!(options.raw(key).is_some(), "{key}");
                }
                ClientOptionEncoding::TextList | ClientOptionEncoding::TutorialStep => {
                    assert!(options.raw(key).is_some(), "{key}");
                }
            }
        }
    }
}

#[cfg(test)]
mod report_tests {
    use super::*;

    #[test]
    fn the_report_dump_covers_the_dumped_block_and_the_keys_it_names() {
        let options = ClientOptions::defaults(ClientOptionHost::default());

        let dump = options.dump_for_report();

        // `processDumpedOptions` names 37 keys and the dump adds 7 more, one
        // of which (`fullscreenResolution`) `save` writes separately and so is
        // not a catalogued line.
        let dumped = CLIENT_OPTIONS.iter().filter(|spec| spec.dumped).count();
        let extras = CLIENT_OPTION_REPORT_EXTRA_KEYS
            .iter()
            .filter(|key| client_option_spec(key).is_some_and(|spec| !spec.dumped))
            .count();
        assert_eq!(dump.len(), dumped + extras + 1);

        let keys: Vec<&str> = dump.iter().map(|(key, _)| key.as_str()).collect();
        // Vanilla sorts by key before joining.
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
        assert!(keys.contains(&"ao"));
        assert!(keys.contains(&"renderDistance"));
        assert!(keys.contains(&"resourcePacks"));
        assert!(keys.contains(&"useNativeTransport"));
        // A key only `processOptions` names stays out of the dump.
        assert!(!keys.contains(&"autoJump"));
        // `fullscreenResolution` is always present exactly once, with the
        // literal `"null"` while no preferred mode is stored.
        assert_eq!(
            keys.iter()
                .filter(|key| **key == "fullscreenResolution")
                .count(),
            1
        );

        let values: Vec<&str> = dump.iter().map(|(_, value)| value.as_str()).collect();
        assert_eq!(
            values[keys.iter().position(|key| *key == "ao").unwrap()],
            "true"
        );
        assert_eq!(
            values[keys
                .iter()
                .position(|key| *key == "fullscreenResolution")
                .unwrap()],
            "null"
        );
    }
}
