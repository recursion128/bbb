//! `options.txt` values feeding the startup arguments they own.
//!
//! Vanilla has no command line for these — `Options` is the only source. BBB
//! reaches the same values through startup arguments, so the file is the base
//! and an argument the user actually typed wins over it. An argument left at
//! its clap default is not a choice, so the stored line takes it.
//!
//! Every stored line is validated by the same `value_parser` the command line
//! uses, and a line that fails keeps the current value — which is what
//! `FieldAccess.process` does when a number does not parse.

use clap::parser::ValueSource;
use clap::ArgMatches;

use super::{client_option_spec, ClientOptionEncoding, ClientOptions};

/// One argument that mirrors an `options.txt` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientOptionBinding {
    pub(crate) arg_id: &'static str,
    pub(crate) key: &'static str,
}

/// Reads `key` off the store and hands the raw line to `parse`, keeping the
/// current value when the argument was typed or the line does not validate.
pub(crate) struct StoredOptions<'a> {
    options: &'a ClientOptions,
    explicit: Box<dyn Fn(&str) -> bool + 'a>,
    bound: Vec<ClientOptionBinding>,
}

impl<'a> StoredOptions<'a> {
    pub(crate) fn new(options: &'a ClientOptions, matches: &'a ArgMatches) -> Self {
        Self {
            options,
            explicit: Box::new(move |arg_id| {
                matches.value_source(arg_id) == Some(ValueSource::CommandLine)
            }),
            bound: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn without_command_line(options: &'a ClientOptions) -> Self {
        Self {
            options,
            explicit: Box::new(|_| false),
            bound: Vec::new(),
        }
    }

    pub(crate) fn is_explicit(&self, arg_id: &str) -> bool {
        (self.explicit)(arg_id)
    }

    /// Overwrite `target` from `key` unless the user typed `arg_id`.
    ///
    /// `expected` is the encoding `save` writes for that key; binding a key
    /// through a parser for a different shape is a bug, so it panics rather
    /// than quietly reading the wrong text.
    pub(crate) fn bind<T>(
        &mut self,
        arg_id: &'static str,
        key: &'static str,
        expected: ClientOptionEncoding,
        target: &mut T,
        parse: impl Fn(&str) -> std::result::Result<T, String>,
    ) {
        let spec = client_option_spec(key)
            .unwrap_or_else(|| panic!("{arg_id} binds unknown options.txt key {key}"));
        assert_eq!(
            spec.encoding, expected,
            "{arg_id} binds {key}, which save writes as {:?}",
            spec.encoding
        );
        self.bound.push(ClientOptionBinding { arg_id, key });
        if (self.explicit)(arg_id) {
            return;
        }
        let Some(raw) = self.options.raw(key) else {
            return;
        };
        match parse(raw) {
            Ok(value) => *target = value,
            Err(error) => {
                tracing::warn!(key, raw, %error, "ignoring invalid options.txt value");
            }
        }
    }

    /// Binds the one special startup line (`fullscreenResolution`,
    /// Options.java:1676/1759-1761) that `Options.load` reads and
    /// `Options.save` writes outside the fixed catalog. No other
    /// `dumpOptionsForReport` field is a persisted options line, so this
    /// binding carries no encoding assertion beyond the parser.
    pub(crate) fn bind_fullscreen_resolution<T>(
        &mut self,
        arg_id: &'static str,
        target: &mut Option<T>,
        parse: impl Fn(&str) -> std::result::Result<T, String>,
    ) {
        let key = super::FULLSCREEN_RESOLUTION_KEY;
        self.bound.push(ClientOptionBinding { arg_id, key });
        if (self.explicit)(arg_id) {
            return;
        }
        let Some(raw) = self.options.raw(key) else {
            return;
        };
        match parse(raw) {
            Ok(value) => *target = Some(value),
            Err(error) => {
                tracing::warn!(key, raw, %error, "ignoring invalid options.txt value");
            }
        }
    }

    pub(crate) fn bound(self) -> Vec<ClientOptionBinding> {
        self.bound
    }
}

/// `FieldAccess.process(String, boolean)` treats anything but `true` as false,
/// but the `OptionInstance` overload runs `Codec.BOOL`, which rejects it. Only
/// the two literals round-trip through `save`, so anything else is invalid.
pub(crate) fn parse_stored_boolean(value: &str) -> std::result::Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("expected true or false, got {other}")),
    }
}

/// `fov` stores the FOV in degrees but writes `(degrees - 70.0) / 40.0`, so a
/// stored line has to come back through the codec's read half.
pub(crate) fn stored_fov_degrees(value: &str) -> std::result::Result<u16, String> {
    let scaled: f64 = value
        .parse()
        .map_err(|_| format!("expected a number, got {value}"))?;
    let degrees = (scaled * 40.0 + 70.0) as i64;
    u16::try_from(degrees).map_err(|_| format!("fov {degrees} is out of range"))
}

#[cfg(test)]
mod tests {
    use super::super::ClientOptionHost;
    use super::*;

    #[test]
    fn a_typed_argument_wins_over_the_stored_line() {
        let options = ClientOptions::defaults(ClientOptionHost::default());
        let mut stored = StoredOptions {
            options: &options,
            explicit: Box::new(|arg_id| arg_id == "typed"),
            bound: Vec::new(),
        };
        let mut typed = 1u32;
        let mut untyped = 1u32;

        let integer = |raw: &str| raw.parse().map_err(|_| "bad".to_owned());
        stored.bind(
            "typed",
            "renderDistance",
            ClientOptionEncoding::Integer,
            &mut typed,
            integer,
        );
        stored.bind(
            "untyped",
            "renderDistance",
            ClientOptionEncoding::Integer,
            &mut untyped,
            integer,
        );

        assert_eq!(typed, 1);
        assert_eq!(untyped, 12);
    }

    #[test]
    fn an_invalid_stored_line_keeps_the_current_value() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("renderDistance", "not a number");
        let mut stored = StoredOptions::without_command_line(&options);
        let mut distance = 8u32;

        stored.bind(
            "render_distance",
            "renderDistance",
            ClientOptionEncoding::Integer,
            &mut distance,
            |raw| raw.parse().map_err(|_| "bad".to_owned()),
        );

        assert_eq!(distance, 8);
    }

    #[test]
    fn stored_booleans_accept_only_the_two_literals_save_writes() {
        assert_eq!(parse_stored_boolean("true"), Ok(true));
        assert_eq!(parse_stored_boolean("false"), Ok(false));
        assert!(parse_stored_boolean("1").is_err());
        assert!(parse_stored_boolean("").is_err());
    }

    #[test]
    fn stored_fov_comes_back_through_the_codec() {
        // `save` writes `(70 - 70.0) / 40.0` for the default 70 degrees.
        assert_eq!(stored_fov_degrees("0.0"), Ok(70));
        assert_eq!(stored_fov_degrees("1.0"), Ok(110));
        assert_eq!(stored_fov_degrees("-1.0"), Ok(30));
        assert!(stored_fov_degrees("nonsense").is_err());
    }

    #[test]
    fn fullscreen_resolution_binds_through_the_dedicated_raw_value() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("fullscreenResolution", "1920x1080@144:24");
        let mut stored = StoredOptions::without_command_line(&options);
        let mut preferred = None;

        stored.bind_fullscreen_resolution("client_fullscreen_resolution", &mut preferred, |raw| {
            Ok(raw.to_owned())
        });

        assert_eq!(preferred.as_deref(), Some("1920x1080@144:24"));
    }

    #[test]
    #[should_panic(expected = "binds unknown options.txt key")]
    fn binding_a_key_the_catalog_does_not_have_fails_closed() {
        let options = ClientOptions::defaults(ClientOptionHost::default());
        let mut stored = StoredOptions::without_command_line(&options);
        let mut sink = 0u32;

        stored.bind(
            "made_up",
            "noSuchOption",
            ClientOptionEncoding::Integer,
            &mut sink,
            |raw| raw.parse().map_err(|_| "bad".to_owned()),
        );
    }

    #[test]
    #[should_panic(expected = "save writes as")]
    fn binding_a_key_through_the_wrong_encoding_fails_closed() {
        let options = ClientOptions::defaults(ClientOptionHost::default());
        let mut stored = StoredOptions::without_command_line(&options);
        let mut sink = false;

        stored.bind(
            "render_distance",
            "renderDistance",
            ClientOptionEncoding::Boolean,
            &mut sink,
            parse_stored_boolean,
        );
    }
}
