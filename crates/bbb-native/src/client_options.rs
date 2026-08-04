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
pub(crate) mod derived;

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

/// The client's `options.txt` state: every catalogued key, plus the per-element
/// lines the three trailing loops own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientOptions {
    values: BTreeMap<&'static str, String>,
    /// Lines under `key_`, `soundCategory_` and `modelPart_`. BBB has no
    /// `KeyMapping` catalog to rewrite them from, so they are carried through
    /// verbatim rather than dropped — the same bytes Vanilla would write when
    /// nothing changes them, instead of silently discarding a shared game
    /// directory's bindings.
    dynamic: BTreeMap<String, String>,
}

impl ClientOptions {
    /// The state Vanilla starts from when `options.txt` does not exist.
    pub(crate) fn defaults(host: ClientOptionHost) -> Self {
        Self {
            values: CLIENT_OPTIONS
                .iter()
                .map(|spec| (spec.key, default_text(spec, host).to_owned()))
                .collect(),
            dynamic: BTreeMap::new(),
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
        for line in raw.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            options.set_raw(key, value);
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
        out
    }

    /// `Options.dumpOptionsForReport`: the `processDumpedOptions` block plus
    /// the keys the dump names itself, sorted by key. A crash report carries
    /// this so a bad frame can be read against the settings that produced it.
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
            .collect();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        entries.dedup_by(|left, right| left.0 == right.0);
        entries
    }

    /// The text currently on the line for `key`, if the catalog has one.
    pub(crate) fn raw(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
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
        if CLIENT_OPTION_PREFIXES
            .iter()
            .any(|prefix| key.starts_with(prefix))
        {
            self.dynamic.insert(key.to_owned(), value.to_owned());
        }
        // Anything else is a key this client does not know; Vanilla reads it
        // into the raw tag, never consumes it, and drops it on the next save.
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

    /// Apply the lines a hotkey changed, then rewrite the file. Nothing is
    /// written when every value already reads that way, because Vanilla only
    /// reaches `save` from a branch that actually flipped the field.
    pub(crate) fn apply(&mut self, writes: &[ClientOptionWrite]) -> Result<bool> {
        let mut changed = false;
        for write in writes {
            let spec = client_option_spec(write.key)
                .unwrap_or_else(|| panic!("{} is not a catalogued option key", write.key));
            assert_eq!(
                spec.encoding,
                ClientOptionEncoding::Boolean,
                "{} is not written as a boolean",
                write.key
            );
            let text = bool_text(write.value);
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
    pub(crate) value: bool,
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
    fn a_debug_hotkey_toggle_rewrites_the_file_and_survives_a_reload() {
        let path = unique_options_path("hotkey");
        fs::write(&path, "advancedItemTooltips:false\npauseOnLostFocus:true\n").unwrap();
        let mut file = ClientOptionsFile::load(path.clone(), ClientOptionHost::default()).unwrap();

        assert!(file
            .apply(&[
                ClientOptionWrite {
                    key: "advancedItemTooltips",
                    value: true,
                },
                ClientOptionWrite {
                    key: "pauseOnLostFocus",
                    value: false,
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
                value: true,
            }])
            .unwrap());
        assert!(!path.exists());

        assert!(file
            .apply(&[ClientOptionWrite {
                key: "pauseOnLostFocus",
                value: false,
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
            "lang:zh_cn\n\
             someModOption:7\n\
             key_key.attack:key.mouse.left\n\
             soundCategory_master:0.5\n",
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

        // `processDumpedOptions` names 37 keys and the dump adds 7 more, one of
        // which (`fullscreenResolution`) `save` writes separately and so is not
        // a catalogued line.
        let dumped = CLIENT_OPTIONS.iter().filter(|spec| spec.dumped).count();
        let extras = CLIENT_OPTION_REPORT_EXTRA_KEYS
            .iter()
            .filter(|key| client_option_spec(key).is_some_and(|spec| !spec.dumped))
            .count();
        assert_eq!(dump.len(), dumped + extras);

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

        let values: Vec<&str> = dump.iter().map(|(_, value)| value.as_str()).collect();
        assert_eq!(
            values[keys.iter().position(|key| *key == "ao").unwrap()],
            "true"
        );
    }
}
