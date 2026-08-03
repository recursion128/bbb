use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlatformTarget {
    Windows,
    MacOs,
    Unix,
}

impl PlatformTarget {
    const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Unix
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
