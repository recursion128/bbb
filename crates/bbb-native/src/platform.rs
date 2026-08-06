use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use url::Url;

/// The one place a build's operating system turns into behaviour. Classifying
/// from a target name rather than `cfg!` keeps every branch compiled and under
/// test on every host: the macOS and Windows arms below are reachable from the
/// Linux dev host, which is the only host this repository gates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlatformTarget {
    Windows,
    MacOs,
    Unix,
}

impl PlatformTarget {
    fn current() -> Self {
        Self::for_target_os(std::env::consts::OS)
    }

    fn for_target_os(target_os: &str) -> Self {
        match target_os {
            "windows" => Self::Windows,
            "macos" => Self::MacOs,
            _ => Self::Unix,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UriOpenCommand {
    program: &'static str,
    args: Vec<String>,
}

impl UriOpenCommand {
    fn for_target(target: PlatformTarget, url: &str) -> Self {
        match target {
            PlatformTarget::Windows => Self {
                program: "rundll32",
                args: vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()],
            },
            PlatformTarget::MacOs => Self {
                program: "open",
                args: vec![url.to_string()],
            },
            PlatformTarget::Unix => Self {
                program: "xdg-open",
                args: vec![url.to_string()],
            },
        }
    }
}

pub(crate) fn open_uri(value: &str) -> Result<()> {
    validate_http_uri(value)?;
    let command = UriOpenCommand::for_target(PlatformTarget::current(), value);
    let mut child = Command::new(command.program)
        .args(&command.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn {} for URI", command.program))?;
    std::thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => {
            tracing::warn!(?status, "URI opener exited unsuccessfully");
        }
        Err(err) => {
            tracing::warn!(?err, "failed to reap URI opener");
        }
        _ => {}
    });
    Ok(())
}

pub(crate) fn open_path(path: &Path) -> Result<()> {
    let (program, args): (&str, Vec<_>) = match PlatformTarget::current() {
        PlatformTarget::Windows => ("explorer", vec![path.as_os_str()]),
        PlatformTarget::MacOs => ("open", vec![path.as_os_str()]),
        PlatformTarget::Unix => ("xdg-open", vec![path.as_os_str()]),
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn {program} for path {}", path.display()))?;
    std::thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => {
            tracing::warn!(?status, "path opener exited unsuccessfully");
        }
        Err(err) => tracing::warn!(?err, "failed to reap path opener"),
        _ => {}
    });
    Ok(())
}

/// The directory bbb's own client stores sit in under the platform's
/// per-user configuration root.
const CLIENT_STORE_DIRECTORY: &str = "bbb-native";

/// A bbb-owned client store file — one that has no Vanilla `--gameDir`
/// counterpart and therefore needs a per-platform home of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientStore {
    /// Environment variable that names the file outright, ahead of every
    /// platform rule.
    override_var: &'static str,
    /// File name inside the per-platform configuration directory.
    file_name: &'static str,
    /// Relative path used when neither a configuration root nor a home
    /// directory is known.
    working_directory_fallback: &'static str,
}

pub(crate) const CODE_OF_CONDUCT_STORE: ClientStore = ClientStore {
    override_var: "BBB_CODE_OF_CONDUCT_STORE",
    file_name: "code-of-conduct.json",
    working_directory_fallback: "bbb-code-of-conduct.json",
};

pub(crate) const HOTBAR_PRESET_STORE: ClientStore = ClientStore {
    override_var: "BBB_HOTBAR_PRESET_STORE",
    file_name: "hotbar.nbt",
    working_directory_fallback: "bbb-hotbar.nbt",
};

impl ClientStore {
    pub(crate) fn default_path(self) -> PathBuf {
        self.default_path_for(PlatformTarget::current(), std::env::var_os)
    }

    /// `%APPDATA%` on Windows and `~/Library/Application Support` on macOS are
    /// the platform conventions; every other target, and any target whose
    /// platform variable is unset, falls through to the XDG configuration root
    /// and then to `~/.config`.
    fn default_path_for(
        self,
        target: PlatformTarget,
        env: impl Fn(&'static str) -> Option<OsString>,
    ) -> PathBuf {
        if let Some(path) = env(self.override_var) {
            return PathBuf::from(path);
        }
        match target {
            PlatformTarget::Windows => {
                if let Some(app_data) = env("APPDATA") {
                    return self.under(Path::new(&app_data));
                }
            }
            PlatformTarget::MacOs => {
                if let Some(home) = env("HOME") {
                    return self
                        .under(&Path::new(&home).join("Library").join("Application Support"));
                }
            }
            PlatformTarget::Unix => {}
        }
        if let Some(config_home) = env("XDG_CONFIG_HOME") {
            return self.under(Path::new(&config_home));
        }
        if let Some(home) = env("HOME") {
            return self.under(&Path::new(&home).join(".config"));
        }
        PathBuf::from(self.working_directory_fallback)
    }

    fn under(self, configuration_root: &Path) -> PathBuf {
        configuration_root
            .join(CLIENT_STORE_DIRECTORY)
            .join(self.file_name)
    }
}

fn validate_http_uri(value: &str) -> Result<()> {
    if value
        .chars()
        .any(|ch| ch.is_whitespace() || ch.is_control())
    {
        bail!("URI must not contain whitespace or control characters");
    }
    let url = Url::parse(value).with_context(|| format!("invalid URI: {value}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        bail!("URI scheme must be http or https: {}", url.scheme());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_open_commands_match_vanilla_platform_processes() {
        let url = "https://example.com/path?q=hello%20world";
        assert_eq!(
            UriOpenCommand::for_target(PlatformTarget::Windows, url),
            UriOpenCommand {
                program: "rundll32",
                args: vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()],
            }
        );
        assert_eq!(
            UriOpenCommand::for_target(PlatformTarget::MacOs, url),
            UriOpenCommand {
                program: "open",
                args: vec![url.to_string()],
            }
        );
        assert_eq!(
            UriOpenCommand::for_target(PlatformTarget::Unix, url),
            UriOpenCommand {
                program: "xdg-open",
                args: vec![url.to_string()],
            }
        );
    }

    fn environment(pairs: &[(&'static str, &str)]) -> impl Fn(&'static str) -> Option<OsString> {
        let pairs: Vec<(&'static str, OsString)> = pairs
            .iter()
            .map(|(name, value)| (*name, OsString::from(*value)))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(candidate, _)| *candidate == name)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn platform_targets_are_classified_from_the_target_name() {
        assert_eq!(
            PlatformTarget::for_target_os("windows"),
            PlatformTarget::Windows
        );
        assert_eq!(
            PlatformTarget::for_target_os("macos"),
            PlatformTarget::MacOs
        );
        for target_os in ["linux", "freebsd", "android", ""] {
            assert_eq!(
                PlatformTarget::for_target_os(target_os),
                PlatformTarget::Unix,
                "{target_os}"
            );
        }
        assert_eq!(
            PlatformTarget::current(),
            PlatformTarget::for_target_os(std::env::consts::OS)
        );
    }

    #[test]
    fn client_stores_follow_the_platform_configuration_convention() {
        let home = environment(&[("HOME", "/home/player")]);
        assert_eq!(
            CODE_OF_CONDUCT_STORE.default_path_for(PlatformTarget::MacOs, &home),
            Path::new("/home/player/Library/Application Support/bbb-native/code-of-conduct.json")
        );
        assert_eq!(
            HOTBAR_PRESET_STORE.default_path_for(PlatformTarget::MacOs, &home),
            Path::new("/home/player/Library/Application Support/bbb-native/hotbar.nbt")
        );
        assert_eq!(
            HOTBAR_PRESET_STORE.default_path_for(
                PlatformTarget::Windows,
                environment(&[("APPDATA", "C:/Users/player/AppData/Roaming")]),
            ),
            Path::new("C:/Users/player/AppData/Roaming/bbb-native/hotbar.nbt")
        );
        assert_eq!(
            HOTBAR_PRESET_STORE.default_path_for(PlatformTarget::Unix, &home),
            Path::new("/home/player/.config/bbb-native/hotbar.nbt")
        );
        assert_eq!(
            HOTBAR_PRESET_STORE.default_path_for(
                PlatformTarget::Unix,
                environment(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/player")]),
            ),
            Path::new("/xdg/bbb-native/hotbar.nbt")
        );
    }

    #[test]
    fn client_store_overrides_outrank_every_platform_rule() {
        for target in [
            PlatformTarget::Windows,
            PlatformTarget::MacOs,
            PlatformTarget::Unix,
        ] {
            assert_eq!(
                CODE_OF_CONDUCT_STORE.default_path_for(
                    target,
                    environment(&[
                        ("BBB_CODE_OF_CONDUCT_STORE", "/explicit/store.json"),
                        ("HOME", "/home/player"),
                        ("APPDATA", "C:/Users/player/AppData/Roaming"),
                        ("XDG_CONFIG_HOME", "/xdg"),
                    ]),
                ),
                Path::new("/explicit/store.json"),
                "{target:?}"
            );
        }
    }

    /// A platform variable that is not set must not strand the store in the
    /// working directory while a configuration root is still available.
    #[test]
    fn platform_branches_fall_through_when_their_variable_is_unset() {
        let xdg_only = environment(&[("XDG_CONFIG_HOME", "/xdg")]);
        assert_eq!(
            HOTBAR_PRESET_STORE.default_path_for(PlatformTarget::Windows, &xdg_only),
            Path::new("/xdg/bbb-native/hotbar.nbt")
        );
        assert_eq!(
            HOTBAR_PRESET_STORE.default_path_for(PlatformTarget::MacOs, &xdg_only),
            Path::new("/xdg/bbb-native/hotbar.nbt")
        );
        for target in [
            PlatformTarget::Windows,
            PlatformTarget::MacOs,
            PlatformTarget::Unix,
        ] {
            assert_eq!(
                HOTBAR_PRESET_STORE.default_path_for(target, environment(&[])),
                Path::new("bbb-hotbar.nbt"),
                "{target:?}"
            );
            assert_eq!(
                CODE_OF_CONDUCT_STORE.default_path_for(target, environment(&[])),
                Path::new("bbb-code-of-conduct.json"),
                "{target:?}"
            );
        }
    }

    #[test]
    fn url_validation_accepts_only_http_and_https() {
        for value in ["https://example.com", "http://example.com/path"] {
            assert!(validate_http_uri(value).is_ok(), "{value}");
        }
        for value in [
            "ftp://example.com",
            "file:///tmp/example",
            "not a url",
            "https://example.com/a b",
            "https://example.com/\nnext",
        ] {
            assert!(validate_http_uri(value).is_err(), "{value}");
        }
    }
}
