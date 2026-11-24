//! Unified cross-platform end-to-end test dispatcher (`cargo dev e2e`).

use std::{env, path::Path, process::ExitCode};

use crate::toolchain::{
    cargo_command, maybe_prepared_vanilla_root, prepare_python_command, resolve_cargo_bin,
    resolve_target_dir, status_to_exit_code, XtaskError,
};

pub fn print_e2e_usage() {
    eprintln!(
        "\
Usage:
  cargo dev e2e <target> [options...]

Targets:
  server [bbb|live] [--skip-build] [--artifact-dir <dir>] [--timeout-seconds <sec>]
      Run normalized bbb-client <-> bbb-server playable & operator interaction E2E
      (`live` also runs differential comparison against Mojang 26.1 with BBB_PLAYABLE_ACCEPT_EULA=1).

  web [--skip-build] [--headless|--headed] [--software-gpu] [--artifact-dir <dir>]
      Build threaded WASM bundle + bbb-server + bbb-web-host and run Chromium WebGPU + MCP E2E.
      See `cargo dev e2e web --help` for all Web options.

  client [--skip-build] [--software-gpu] [--out <dir>] [--vanilla-root <dir>]
      Build bbb-server + bbb-admin + bbb-client + bbb-replay-diff and run lockstep MCP playable
      gameplay + 2-pass deterministic replay verification (Weston on Linux, native on Windows/macOS).

  server-release [--skip-build] [FAMILY ...] [--artifact-dir <dir>] [--disperse-players]
      Run dedicated-server release acceptance families (concurrency, chunk_load, restart,
      hostile, soak, signal, creation, autosave).

Run `cargo dev e2e <target> --help` for full target-specific options."
    );
}

fn build_packages(root: &Path, cargo_args: &[&str]) -> Result<(), XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let status = cargo_command(root, &cargo_bin, &target_dir)
        .arg("build")
        .arg("--locked")
        .args(cargo_args)
        .status()
        .map_err(|err| XtaskError::failed(format!("cargo build for e2e: {err}")))?;
    if !status.success() {
        return Err(XtaskError::with_code(
            u8::try_from(status.code().unwrap_or(1)).unwrap_or(1),
            format!("cargo build failed with {status}"),
        ));
    }
    Ok(())
}

fn run_e2e_python(root: &Path, script: &str, args: &[String]) -> Result<ExitCode, XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let mut py_args = vec!["python".to_string(), script.to_string()];
    py_args.extend_from_slice(args);
    let mut cmd = prepare_python_command(root, &py_args)?;
    cmd.env("CARGO", &cargo_bin)
        .env("CARGO_TARGET_DIR", &target_dir);
    if env::var_os("BBB_VANILLA_ROOT").is_none() {
        if let Some(vanilla) = maybe_prepared_vanilla_root() {
            cmd.env("BBB_VANILLA_ROOT", vanilla);
        }
    }
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("run {script}: {err}")))?;
    Ok(status_to_exit_code(status))
}

fn extract_skip_build(args: &[String]) -> (bool, Vec<String>) {
    let mut skip_build = false;
    let mut rest = Vec::with_capacity(args.len());
    for arg in args {
        if arg == "--skip-build" {
            skip_build = true;
        } else {
            rest.push(arg.clone());
        }
    }
    (skip_build, rest)
}

pub fn run_e2e(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let Some(target) = args.first().map(String::as_str) else {
        print_e2e_usage();
        return Err(XtaskError::usage(""));
    };
    if matches!(target, "-h" | "--help" | "help") {
        print_e2e_usage();
        return Ok(ExitCode::SUCCESS);
    }

    let sub_args = &args[1..];
    if target == "web" {
        return run_e2e_python(root, "scripts/web_playable_gate.py", sub_args);
    }

    let help_requested = sub_args.iter().any(|a| a == "-h" || a == "--help");
    let (skip_build, forwarded) = extract_skip_build(sub_args);

    match target {
        "server" => {
            if !skip_build && !help_requested {
                build_packages(
                    root,
                    &["-p", "bbb-server", "-p", "bbb-admin", "-p", "bbb-client"],
                )?;
            }
            let mut run_args = vec!["run".to_string()];
            run_args.extend(forwarded);
            run_e2e_python(root, "scripts/server_playable_gate.py", &run_args)
        }
        "client" => {
            if !skip_build && !help_requested {
                build_packages(
                    root,
                    &[
                        "-p",
                        "bbb-server",
                        "-p",
                        "bbb-admin",
                        "-p",
                        "bbb-client",
                        "-p",
                        "bbb-client-probe",
                        "--bins",
                    ],
                )?;
            }
            let mut client_args = Vec::new();
            if !help_requested && !forwarded.iter().any(|a| a == "--target") {
                let debug_dir = resolve_target_dir()?.join("debug");
                client_args.push("--target".to_string());
                client_args.push(debug_dir.display().to_string());
            }
            client_args.extend(forwarded);
            run_e2e_python(root, "scripts/mcp_playable_gate.py", &client_args)
        }
        "server-release" => {
            if !skip_build && !help_requested {
                build_packages(root, &["--release", "-p", "bbb-server"])?;
                build_packages(root, &["-p", "bbb-admin", "-p", "bbb-client"])?;
            }
            run_e2e_python(root, "scripts/server_release_gate.py", &forwarded)
        }
        other => Err(XtaskError::usage(format!(
            "unknown e2e target '{other}' (expected server, web, client, or server-release)"
        ))),
    }
}
