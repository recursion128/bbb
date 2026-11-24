//! Desktop client build-and-launch helper (`cargo dev client`).

use std::{env, path::Path, process::ExitCode};

use crate::toolchain::{
    cargo_command, maybe_prepared_vanilla_root, resolve_cargo_bin, resolve_target_dir,
    status_to_exit_code, XtaskError, MIN_TEST_STACK_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientLaunchOptions {
    pub release: bool,
    pub passthrough: Vec<String>,
}

pub fn print_client_usage() {
    eprintln!(
        "\
Usage:
  cargo dev client [--release] [--] [bbb-client args...]

Builds and runs the native desktop `bbb-client` binary with `BBB_VANILLA_ROOT`
populated when a prepared Vanilla 26.1 cache is present, forwarding all client
flags verbatim.

Options:
  --release         Build and run `bbb-client` with `--release`
  -h, --help        Show this help message (pass `-- --help` for `bbb-client --help`)"
    );
}

pub fn parse_client_args(
    args: &[String],
    print_usage: fn(),
) -> Result<Option<ClientLaunchOptions>, XtaskError> {
    if args.first().map(String::as_str) == Some("help") {
        print_usage();
        return Ok(None);
    }

    let mut release = false;
    let mut passthrough = Vec::new();
    let mut idx = 0;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--" {
            passthrough.extend_from_slice(&args[idx + 1..]);
            break;
        }
        if matches!(arg.as_str(), "-h" | "--help") {
            print_usage();
            return Ok(None);
        }
        if arg == "--release" {
            release = true;
        } else {
            passthrough.push(arg.clone());
        }
        idx += 1;
    }

    Ok(Some(ClientLaunchOptions {
        release,
        passthrough,
    }))
}

pub fn build_client_cargo_args(opts: &ClientLaunchOptions) -> Vec<String> {
    let mut args = vec!["run".to_string(), "--locked".to_string()];
    if opts.release {
        args.push("--release".to_string());
    }
    args.extend([
        "-p".to_string(),
        "bbb-client".to_string(),
        "--bin".to_string(),
        "bbb-client".to_string(),
    ]);
    if !opts.passthrough.is_empty() {
        args.push("--".to_string());
        args.extend(opts.passthrough.iter().cloned());
    }
    args
}

pub fn run_client(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let Some(opts) = parse_client_args(args, print_client_usage)? else {
        return Ok(ExitCode::SUCCESS);
    };
    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let mut cmd = cargo_command(root, &cargo_bin, &target_dir);
    if env::var_os("RUST_MIN_STACK").is_none() {
        cmd.env("RUST_MIN_STACK", MIN_TEST_STACK_BYTES);
    }
    if env::var_os("BBB_VANILLA_ROOT").is_none() {
        if let Some(vanilla) = maybe_prepared_vanilla_root() {
            cmd.env("BBB_VANILLA_ROOT", vanilla);
        }
    }
    cmd.args(build_client_cargo_args(&opts));
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("invoke cargo run bbb-client: {err}")))?;
    Ok(status_to_exit_code(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop() {}

    #[test]
    fn help_flags_exit_without_building() {
        for flag in ["-h", "--help", "help"] {
            let parsed = parse_client_args(&[flag.to_string()], noop).unwrap();
            assert!(parsed.is_none(), "expected {flag} to return None");
        }
        let with_release =
            parse_client_args(&["--release".to_string(), "--help".to_string()], noop).unwrap();
        assert!(with_release.is_none());
    }

    #[test]
    fn forwards_client_flags_and_handles_release_and_separator() {
        let empty = parse_client_args(&[], noop).unwrap().unwrap();
        assert!(!empty.release);
        assert!(empty.passthrough.is_empty());
        assert_eq!(
            build_client_cargo_args(&empty),
            vec!["run", "--locked", "-p", "bbb-client", "--bin", "bbb-client"]
        );

        let forwarded = parse_client_args(
            &[
                "--release".to_string(),
                "--server".to_string(),
                "127.0.0.1:25565".to_string(),
                "--fullscreen".to_string(),
            ],
            noop,
        )
        .unwrap()
        .unwrap();
        assert!(forwarded.release);
        assert_eq!(
            build_client_cargo_args(&forwarded),
            vec![
                "run",
                "--locked",
                "--release",
                "-p",
                "bbb-client",
                "--bin",
                "bbb-client",
                "--",
                "--server",
                "127.0.0.1:25565",
                "--fullscreen",
            ]
        );

        let after_sep = parse_client_args(&["--".to_string(), "--help".to_string()], noop)
            .unwrap()
            .unwrap();
        assert!(!after_sep.release);
        assert_eq!(
            build_client_cargo_args(&after_sep),
            vec![
                "run",
                "--locked",
                "-p",
                "bbb-client",
                "--bin",
                "bbb-client",
                "--",
                "--help",
            ]
        );
    }
}
