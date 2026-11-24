//! Cross-platform developer environment bootstrap (`cargo dev bootstrap`).

use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use crate::toolchain::{
    cargo_command, exe_suffix, find_in_path, host_triple_from_rustc, path_with_toolchain_bin,
    prepare_python_command, resolve_cargo_bin, resolve_target_dir, run_python_tool, XtaskError,
};

const REQUIRED_UV_VERSION: &str = "0.12.3";
const DEFAULT_CROSS_TARGET: &str = "aarch64-apple-darwin";
const DARWIN_CROSS_TARGET: &str = "x86_64-unknown-linux-gnu";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BootstrapOptions {
    offline: bool,
    prepare_vanilla: bool,
    skip_cross_target: bool,
}

fn default_cross_target_for_host(host_target: &str) -> &'static str {
    if host_target.contains("apple-darwin") {
        DARWIN_CROSS_TARGET
    } else {
        DEFAULT_CROSS_TARGET
    }
}

pub fn print_bootstrap_usage() {
    eprintln!(
        "\
usage: cargo dev bootstrap [--offline] [--skip-vanilla] [--skip-cross-target]

Prepare the pinned uv-managed Python, locked Python and Rust dependencies, and
the managed Vanilla 26.1 workspace. Use --skip-vanilla when only the developer
toolchains are needed; --skip-cross-target skips the second-platform cross-check
Rust target; --offline requires every artifact to be cached already."
    );
}

fn parse_bootstrap_args(args: &[String]) -> Result<Option<BootstrapOptions>, XtaskError> {
    let mut opts = BootstrapOptions {
        offline: false,
        prepare_vanilla: true,
        skip_cross_target: false,
    };

    for arg in args {
        match arg.as_str() {
            "--offline" => opts.offline = true,
            "--skip-vanilla" => opts.prepare_vanilla = false,
            "--skip-cross-target" => opts.skip_cross_target = true,
            "-h" | "--help" => {
                print_bootstrap_usage();
                return Ok(None);
            }
            other => {
                print_bootstrap_usage();
                return Err(XtaskError::usage(format!(
                    "unknown bootstrap option: {other}"
                )));
            }
        }
    }
    Ok(Some(opts))
}

pub fn run_bootstrap(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let Some(opts) = parse_bootstrap_args(args)? else {
        return Ok(ExitCode::SUCCESS);
    };
    let BootstrapOptions {
        offline,
        prepare_vanilla,
        skip_cross_target,
    } = opts;

    let uv_bin = find_in_path("uv").ok_or_else(|| {
        XtaskError::usage(format!(
            "bootstrap requires uv {REQUIRED_UV_VERSION}; install that version and rerun"
        ))
    })?;
    let uv_ver_out = Command::new(&uv_bin)
        .args(["--no-config", "--version"])
        .output()
        .map_err(|err| XtaskError::failed(format!("run uv --version: {err}")))?;
    let uv_ver_str = String::from_utf8_lossy(&uv_ver_out.stdout);
    let installed_uv = uv_ver_str.split_whitespace().nth(1).unwrap_or("");
    if installed_uv != REQUIRED_UV_VERSION {
        return Err(XtaskError::usage(format!(
            "bootstrap requires uv {REQUIRED_UV_VERSION}; found {installed_uv}\n\
             standalone uv installs can switch with: uv --no-config self update {REQUIRED_UV_VERSION}"
        )));
    }

    let cargo_bin = resolve_cargo_bin()?;
    for tool in ["rustc", "rustup", "git"] {
        let present = find_in_path(tool).is_some()
            || cargo_bin
                .parent()
                .map(|d| d.join(format!("{tool}{}", exe_suffix())).exists())
                .unwrap_or(false);
        if !present {
            return Err(XtaskError::usage(format!(
                "bootstrap requires {tool} on PATH"
            )));
        }
    }

    println!("== pinned Python ==");
    let mut uv_py = Command::new(&uv_bin);
    uv_py.arg("--project").arg(root).args(["python", "install"]);
    if offline {
        uv_py.arg("--offline");
    }
    let status = uv_py
        .status()
        .map_err(|err| XtaskError::failed(format!("uv python install: {err}")))?;
    if !status.success() {
        return Err(XtaskError::failed("uv python install failed"));
    }

    println!("== locked Python dependencies ==");
    let mut uv_sync = Command::new(&uv_bin);
    uv_sync
        .arg("--project")
        .arg(root)
        .args(["sync", "--locked"]);
    if offline {
        uv_sync.arg("--offline");
    }
    let status = uv_sync
        .status()
        .map_err(|err| XtaskError::failed(format!("uv sync: {err}")))?;
    if !status.success() {
        return Err(XtaskError::failed("uv sync --locked failed"));
    }

    println!("== portable host capabilities ==");
    let doctor_args = vec![
        "python".to_string(),
        "-m".to_string(),
        "bbb_tools.dev_cli".to_string(),
        "doctor".to_string(),
        "--profile".to_string(),
        "portable".to_string(),
    ];
    let code = run_python_tool(root, &doctor_args)?;
    if code != ExitCode::SUCCESS {
        return Ok(code);
    }

    println!("== locked Rust dependencies ==");
    let target_dir = resolve_target_dir()?;
    let host_target = host_triple_from_rustc(&cargo_bin)?;
    let cross_target = default_cross_target_for_host(&host_target);
    let mut cross_ready = false;

    if !skip_cross_target && host_target != cross_target {
        let installed_targets = installed_rustup_targets(&cargo_bin);
        cross_ready = installed_targets.iter().any(|t| t == cross_target);
        if !cross_ready {
            if offline {
                return Err(XtaskError::usage(format!(
                    "offline bootstrap requires the cached Rust target {cross_target}\n\
                     install it online with: rustup target add {cross_target}"
                )));
            }
            let rustup_bin = resolve_rustup_bin(&cargo_bin);
            let status = Command::new(&rustup_bin)
                .env("PATH", path_with_toolchain_bin(&cargo_bin))
                .args(["target", "add", cross_target])
                .status()
                .map_err(|err| XtaskError::failed(format!("rustup target add: {err}")))?;
            if !status.success() {
                return Err(XtaskError::failed(format!(
                    "rustup target add {cross_target} failed"
                )));
            }
            cross_ready = true;
        }
    }

    cargo_fetch_target(root, &cargo_bin, &target_dir, &host_target, offline)?;
    if !skip_cross_target && cross_ready && host_target != cross_target {
        cargo_fetch_target(root, &cargo_bin, &target_dir, cross_target, offline)?;
    }

    if prepare_vanilla {
        let state_check = prepare_python_command(
            root,
            &[
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.vanilla.cli".to_string(),
                "path".to_string(),
                "26.1".to_string(),
                "--kind".to_string(),
                "state".to_string(),
            ],
        )?
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

        if state_check {
            println!("== managed Vanilla 26.1 workspace ==");
            let verify_args = vec![
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.vanilla.cli".to_string(),
                "verify".to_string(),
                "26.1".to_string(),
            ];
            let code = run_python_tool(root, &verify_args)?;
            if code != ExitCode::SUCCESS {
                return Ok(code);
            }
        } else {
            if find_in_path("java").is_none() {
                return Err(XtaskError::usage(
                    "Vanilla 26.1 preparation requires Java 25 on PATH\n\
                     install Java 25, or rerun with --skip-vanilla for toolchains only",
                ));
            }
            println!("== managed Vanilla 26.1 workspace ==");
            let mut prep_args = vec![
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.vanilla.cli".to_string(),
                "prepare".to_string(),
                "26.1".to_string(),
            ];
            if offline {
                prep_args.push("--offline".to_string());
            }
            let code = run_python_tool(root, &prep_args)?;
            if code != ExitCode::SUCCESS {
                return Ok(code);
            }
            let verify_args = vec![
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.vanilla.cli".to_string(),
                "verify".to_string(),
                "26.1".to_string(),
            ];
            let code = run_python_tool(root, &verify_args)?;
            if code != ExitCode::SUCCESS {
                return Ok(code);
            }
        }
    }

    if prepare_vanilla {
        println!("bootstrap: ready; run cargo dev gate");
    } else {
        println!("bootstrap: ready; run cargo dev lint --skip-vanilla");
    }
    Ok(ExitCode::SUCCESS)
}

fn resolve_rustup_bin(cargo_bin: &Path) -> PathBuf {
    let bin_name = format!("rustup{}", exe_suffix());
    cargo_bin
        .parent()
        .map(|d| d.join(&bin_name))
        .filter(|p| p.is_file())
        .or_else(|| find_in_path("rustup"))
        .unwrap_or_else(|| PathBuf::from(bin_name))
}

fn installed_rustup_targets(cargo_bin: &Path) -> Vec<String> {
    let rustup_bin = resolve_rustup_bin(cargo_bin);
    let Ok(out) = Command::new(rustup_bin)
        .env("PATH", path_with_toolchain_bin(cargo_bin))
        .args(["target", "list", "--installed"])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn cargo_fetch_target(
    root: &Path,
    cargo_bin: &Path,
    target_dir: &Path,
    target: &str,
    offline: bool,
) -> Result<(), XtaskError> {
    let mut cmd = cargo_command(root, cargo_bin, target_dir);
    cmd.arg("fetch")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .args(["--locked", "--target", target]);
    if offline {
        cmd.arg("--offline");
    }
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("cargo fetch --target {target}: {err}")))?;
    if !status.success() {
        return Err(XtaskError::failed(format!(
            "cargo fetch --target {target} exited with {status}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cross_target_is_symmetric_for_darwin_and_non_darwin_hosts() {
        assert_eq!(
            default_cross_target_for_host("aarch64-apple-darwin"),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            default_cross_target_for_host("x86_64-apple-darwin"),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            default_cross_target_for_host("x86_64-unknown-linux-gnu"),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            default_cross_target_for_host("x86_64-pc-windows-msvc"),
            "aarch64-apple-darwin"
        );
    }

    #[test]
    fn parse_bootstrap_args_handles_skip_cross_target_and_help() {
        assert_eq!(
            parse_bootstrap_args(&[]).unwrap(),
            Some(BootstrapOptions {
                offline: false,
                prepare_vanilla: true,
                skip_cross_target: false,
            })
        );
        assert_eq!(
            parse_bootstrap_args(&[
                "--offline".to_string(),
                "--skip-vanilla".to_string(),
                "--skip-cross-target".to_string(),
            ])
            .unwrap(),
            Some(BootstrapOptions {
                offline: true,
                prepare_vanilla: false,
                skip_cross_target: true,
            })
        );
        assert_eq!(parse_bootstrap_args(&["--help".to_string()]).unwrap(), None);
        assert!(parse_bootstrap_args(&["--bogus".to_string()]).is_err());
    }
}
