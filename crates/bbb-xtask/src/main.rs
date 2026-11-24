//! Unified cross-platform developer and testing workflow runner (`cargo dev`).
//!
//! Owns Cargo target-directory hygiene, focused and full gate testing, worker
//! worktree and warm-pool management, Python/Vanilla tool invocation, and Web
//! WASM packaging + local stack orchestration across Linux, macOS, and Windows
//! with zero external crate dependencies.

mod bench;
mod bisect;
mod bootstrap;
mod cache;
mod client;
mod e2e;
mod gate;
mod toolchain;
mod web;
mod worker;

use std::{env, path::Path, process::ExitCode};

use bench::run_bench;
use bisect::run_batch_gate;
use bootstrap::run_bootstrap;
use cache::{run_clean_target, run_prune, run_size};
use client::run_client;
use e2e::run_e2e;
use gate::{run_check_targets, run_gate, run_install_hooks, run_lint, run_pre_commit};
use toolchain::{
    cargo_command, host_triple_from_rustc, maybe_prepared_vanilla_root, path_with_toolchain_bin,
    prepare_python_command, repo_root, resolve_cargo_bin, resolve_target_dir, resolve_vanilla_root,
    run_python_tool, status_to_exit_code, XtaskError, MIN_TEST_STACK_BYTES,
};
use web::{
    build_web, default_wasm_target_dir, default_web_out_dir, parse_build_web_args,
    parse_web_up_args, print_build_web_usage, print_web_up_usage, run_web_up,
};
use worker::run_worker_cli;

fn main() -> ExitCode {
    match dispatch(env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(err) => {
            if !err.message().is_empty() {
                eprintln!("{}", err.message());
            }
            err.exit_code()
        }
    }
}

pub fn print_usage() {
    eprintln!(
        "\
Usage:
  cargo dev <command> [args...]
  cargo worker <create|run|status|cleanup|release|env|shell-env> [args...]
  cargo build-web [--dev|--release] [--out-dir <dir>] [--wasm-target-dir <dir>]
  cargo web-up [options]

Rust build & test:
  build [cargo args...]       cargo build --locked with shared target dir
  check [cargo args...]       cargo check --locked with shared target dir
  test [cargo args...]        cargo test --locked with shared target dir
  fast-test [cargo args...]   cargo test --locked --profile fast-test
  bench [name|args...]        run Criterion, server, or replay benchmarks
  run [cargo args...]         cargo run --locked with shared target dir
  clippy [cargo args...]      cargo clippy --locked with shared target dir
  fmt [cargo args...]         cargo fmt --all [args...]
  cargo [cargo args...]       raw cargo command with shared target dir
  env                         print resolved cross-platform paths & env

Tooling, lints & gates:
  bootstrap [--offline] [--skip-vanilla]   prepare Python, Rust & Vanilla 26.1
  vanilla <args...>                        manage Vanilla 26.1 workspace
  python <cmd-or-script> [args...]         run managed Python in primary .venv
  world-build [options]                    build and verify a local world through MCP
  py-test [args...]                        run scripts Python test suites
  py-lint                                  run ruff on Python tooling
  doctor [--profile <name>]                verify host tool capabilities
  generate --check [name]                  check generated catalogs (name or `all`)
  docs-lint [--all-generators] [--skip-vanilla]   run repository & Wiki lints
  audit-p0                                 run P0 allowlist classification audit
  anchors [--all]                          verify Vanilla source line anchors
  lint [--skip-vanilla]                    run fast repo/Rust/Python/Wiki lints
  cross-check [--target <triple>]          type-check second-platform crates
  check-targets                            check host + wasm32 + second-platform
  e2e <server|web|client|server-release>   run unified end-to-end playable gates
  gate [--dry-run] [--skip-vanilla]        run full (or portable) gate steps & stamp
  batch-gate [<good>..<bad>]               bisect first failing commit in range
  install-hooks                            install .git/hooks/pre-commit
  pre-commit                               run pre-commit checks

Cache hygiene & worker lanes:
  prune [--force]                          prune disposable target directories
  size                                     print bbb-target-* directory sizes
  clean-target <name>                      delete one bbb-target-<name> cache
  worker <subcommand> [args...]            manage isolated worker worktrees

Desktop & Web client:
  client [--release] [--] [args...]        build & run desktop bbb-client
  build-web [options]                      build threaded WASM client bundle
  web-up [options]                         start bbb-server + bbb-web-host"
    );
}

fn dispatch(args: Vec<String>) -> Result<ExitCode, XtaskError> {
    let Some(cmd) = args.first().map(String::as_str) else {
        print_usage();
        return Err(XtaskError::usage(""));
    };
    if matches!(cmd, "-h" | "--help" | "help") {
        print_usage();
        return Ok(ExitCode::SUCCESS);
    }

    let root = repo_root();
    let rest = &args[1..];
    match cmd {
        "build" => run_cargo_locked(&root, "build", rest, false, false),
        "check" => run_cargo_locked(&root, "check", rest, false, false),
        "test" => run_cargo_locked(&root, "test", rest, true, true),
        "fast-test" => {
            let mut forwarded = vec!["--profile".to_string(), "fast-test".to_string()];
            forwarded.extend_from_slice(rest);
            run_cargo_locked(&root, "test", &forwarded, true, true)
        }
        "bench" => run_bench(&root, rest),
        "client" => run_client(&root, rest),
        "run" => run_cargo_locked(&root, "run", rest, false, false),
        "clippy" => run_cargo_locked(&root, "clippy", rest, false, false),
        "fmt" => {
            let mut forwarded = vec!["fmt".to_string()];
            if !rest
                .iter()
                .any(|a| a == "-p" || a == "--package" || a == "--all")
            {
                forwarded.push("--all".to_string());
            }
            forwarded.extend_from_slice(rest);
            run_raw_cargo(&root, &forwarded)
        }
        "cargo" => run_raw_cargo(&root, rest),
        "env" => {
            print_env()?;
            Ok(ExitCode::SUCCESS)
        }
        "bootstrap" => run_bootstrap(&root, rest),
        "vanilla" => {
            let mut py_args = vec![
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.vanilla.cli".to_string(),
            ];
            py_args.extend_from_slice(rest);
            run_python_tool(&root, &py_args)
        }
        "python" => run_python_tool(&root, rest),
        "world-build" => {
            let mut py_args = vec!["scripts/mcp_world_build.py".to_string()];
            py_args.extend_from_slice(rest);
            let status = prepare_python_command(&root, &py_args)?
                .env(
                    "BBB_WORLD_BUILD_BIN_DIR",
                    resolve_target_dir()?.join("debug"),
                )
                .status()
                .map_err(|err| XtaskError::failed(format!("invoke world builder: {err}")))?;
            Ok(status_to_exit_code(status))
        }
        "py-test" => {
            let mut py_args = vec![
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.dev_cli".to_string(),
                "test".to_string(),
                "python".to_string(),
            ];
            py_args.extend_from_slice(rest);
            run_python_tool(&root, &py_args)
        }
        "py-lint" => run_python_tool(
            &root,
            &[
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.dev_cli".to_string(),
                "lint".to_string(),
                "python".to_string(),
            ],
        ),
        "doctor" => {
            let mut py_args = vec![
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.dev_cli".to_string(),
                "doctor".to_string(),
            ];
            py_args.extend_from_slice(rest);
            run_python_tool(&root, &py_args)
        }
        "generate" => {
            let mut py_args = vec![
                "python".to_string(),
                "-m".to_string(),
                "bbb_tools.dev_cli".to_string(),
                "generate".to_string(),
            ];
            py_args.extend_from_slice(rest);
            run_python_tool(&root, &py_args)
        }
        "docs-lint" => {
            let mut py_args = vec!["python".to_string(), "scripts/docs_lint.py".to_string()];
            py_args.extend_from_slice(rest);
            run_python_tool(&root, &py_args)
        }
        "audit-p0" => {
            let mut py_args = vec!["python".to_string(), "scripts/audit_p0.py".to_string()];
            py_args.extend_from_slice(rest);
            run_python_tool(&root, &py_args)
        }
        "anchors" => {
            let mut py_args = vec![
                "python".to_string(),
                "scripts/check_vanilla_anchors.py".to_string(),
            ];
            py_args.extend_from_slice(rest);
            run_python_tool(&root, &py_args)
        }
        "cross-check" => {
            let cargo_bin = resolve_cargo_bin()?;
            let target_dir = resolve_target_dir()?;
            let mut py_args = vec!["python".to_string(), "scripts/cross_check.py".to_string()];
            py_args.extend_from_slice(rest);
            let status = prepare_python_command(&root, &py_args)?
                .env("CARGO", &cargo_bin)
                .env("PATH", path_with_toolchain_bin(&cargo_bin))
                .env("CARGO_TARGET_DIR", &target_dir)
                .status()
                .map_err(|err| XtaskError::failed(format!("invoke cross-check: {err}")))?;
            Ok(status_to_exit_code(status))
        }
        "lint" => run_lint(&root, rest),
        "check-targets" => run_check_targets(&root, rest),
        "e2e" => run_e2e(&root, rest),
        "gate" => run_gate(&root, rest),
        "batch-gate" => run_batch_gate(&root, rest),
        "prune" => run_prune(&root, rest),
        "size" => run_size(rest),
        "clean-target" => run_clean_target(rest),
        "install-hooks" => run_install_hooks(&root),
        "pre-commit" => run_pre_commit(&root),
        "worker" => run_worker_cli(&root, rest),
        "build-web" => {
            if let Some(opts) = parse_build_web_args(rest, print_build_web_usage)? {
                build_web(&root, &opts)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        "web-up" => {
            if let Some(opts) = parse_web_up_args(rest, print_web_up_usage)? {
                run_web_up(&root, &opts)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        other => Err(XtaskError::usage(format!(
            "unknown command '{other}' (try `cargo dev --help`)"
        ))),
    }
}

fn run_cargo_locked(
    root: &Path,
    subcmd: &str,
    rest: &[String],
    set_test_stack: bool,
    export_vanilla: bool,
) -> Result<ExitCode, XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let mut cmd = cargo_command(root, &cargo_bin, &target_dir);
    if set_test_stack && env::var_os("RUST_MIN_STACK").is_none() {
        cmd.env("RUST_MIN_STACK", MIN_TEST_STACK_BYTES);
    }
    if export_vanilla && env::var_os("BBB_VANILLA_ROOT").is_none() {
        if let Some(vanilla) = maybe_prepared_vanilla_root() {
            cmd.env("BBB_VANILLA_ROOT", vanilla);
        }
    }
    cmd.arg(subcmd);
    if !rest
        .iter()
        .take_while(|a| *a != "--")
        .any(|a| a == "--locked")
    {
        cmd.arg("--locked");
    }
    cmd.args(rest);
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("invoke cargo {subcmd}: {err}")))?;
    Ok(status_to_exit_code(status))
}

fn run_raw_cargo(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let mut cmd = cargo_command(root, &cargo_bin, &target_dir);
    cmd.args(args);
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("invoke cargo: {err}")))?;
    Ok(status_to_exit_code(status))
}

fn print_env() -> Result<(), XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let host = host_triple_from_rustc(&cargo_bin)?;
    let vanilla = resolve_vanilla_root(None);
    println!("CARGO={}", cargo_bin.display());
    println!("CARGO_TARGET_DIR={}", target_dir.display());
    println!("HOST_TRIPLE={host}");
    println!("BBB_VANILLA_ROOT={}", vanilla.display());
    println!("BBB_WEB_OUT_DIR={}", default_web_out_dir().display());
    println!(
        "BBB_WASM_TARGET_DIR={}",
        default_wasm_target_dir().display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, ffi::OsString, fs, path::PathBuf};

    use cache::is_prune_keep;
    use toolchain::{
        parse_rustc_host_triple, resolve_vanilla_root_with, target_dir_for_arg_with_root,
        validate_target_suffix,
    };
    use web::parse_locked_package_version;
    use worker::{format_shell_exports, gc_stale_leases, is_pool_slot, LeaseFileLock, ShellKind};

    use super::*;

    #[test]
    fn parses_rustc_host_line() {
        let sample =
            "rustc 1.97.0-nightly\nbinary: rustc\nhost: aarch64-apple-darwin\nrelease: 1.97.0";
        assert_eq!(
            parse_rustc_host_triple(sample).as_deref(),
            Some("aarch64-apple-darwin")
        );
    }

    #[test]
    fn parses_wasm_bindgen_version_from_cargo_lock() {
        let lock = "\
[[package]]
name = \"wasm-bindgen-shared\"
version = \"0.2.100\"

[[package]]
name = \"wasm-bindgen\"
version = \"0.2.127\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
";
        assert_eq!(
            parse_locked_package_version(lock, "wasm-bindgen").as_deref(),
            Some("0.2.127")
        );
    }

    #[test]
    fn validates_target_suffix_pool_slots_and_prune_keep_rules() {
        assert!(validate_target_suffix("main").is_ok());
        assert!(validate_target_suffix("lane-1").is_ok());
        assert!(validate_target_suffix("fast_test").is_ok());
        assert!(validate_target_suffix("").is_err());
        assert!(validate_target_suffix("../escape").is_err());

        assert!(is_pool_slot("lane-1"));
        assert!(is_pool_slot("lane-12"));
        assert!(is_pool_slot("lane-999"));
        assert!(!is_pool_slot("lane-"));
        assert!(!is_pool_slot("lane-1000"));
        assert!(!is_pool_slot("main"));

        for kept in [
            "main",
            "playable",
            "soak",
            "wasm",
            "wasm-bindgen-cli",
            "lane-2",
            "wasm-lane-2",
        ] {
            assert!(is_prune_keep(kept), "expected {kept} to be kept by prune");
        }
        assert!(!is_prune_keep("old-slice"));
        assert!(!is_prune_keep("wasm-old-slice"));
    }

    #[test]
    fn vanilla_root_precedence_matches_platform_cache_conventions() {
        let mut env_map = HashMap::from([
            ("LOCALAPPDATA", OsString::from("/win/local")),
            ("XDG_CACHE_HOME", OsString::from("/xdg/cache")),
            ("HOME", OsString::from("/home/tester")),
        ]);
        assert_eq!(
            resolve_vanilla_root_with(None, "windows", |k| env_map.get(k).cloned()),
            PathBuf::from("/win/local").join("bbb").join("vanilla")
        );
        assert_eq!(
            resolve_vanilla_root_with(None, "macos", |k| env_map.get(k).cloned()),
            PathBuf::from("/home/tester")
                .join("Library")
                .join("Caches")
                .join("bbb")
                .join("vanilla")
        );
        assert_eq!(
            resolve_vanilla_root_with(None, "linux", |k| env_map.get(k).cloned()),
            PathBuf::from("/xdg/cache").join("bbb").join("vanilla")
        );
        env_map.insert("BBB_VANILLA_DIR", OsString::from("/legacy/env"));
        assert_eq!(
            resolve_vanilla_root_with(None, "windows", |k| env_map.get(k).cloned()),
            PathBuf::from("/win/local").join("bbb").join("vanilla")
        );
        env_map.insert("BBB_VANILLA_ROOT", OsString::from("/canonical/env"));
        assert_eq!(
            resolve_vanilla_root_with(None, "windows", |k| env_map.get(k).cloned()),
            PathBuf::from("/canonical/env")
        );
    }

    #[test]
    fn formats_shell_exports_across_sh_powershell_and_cmd_with_optional_vanilla() {
        let wt = Path::new("/tmp/bbb-wt-alpha");
        let target = Path::new("/tmp/bbb-target-lane-1");
        let vanilla = Path::new("/tmp/vanilla");

        let sh_portable = format_shell_exports(ShellKind::Sh, Some(wt), "alpha", target, None);
        assert!(sh_portable.iter().any(|l| l.starts_with("cd ")));
        assert!(sh_portable
            .iter()
            .any(|l| l == "export BBB_CARGO_TARGET_NAME='alpha'"));
        assert!(sh_portable
            .iter()
            .any(|l| l.starts_with("export BBB_WASM_TARGET_DIR=")
                && l.contains("bbb-target-wasm-lane-1")));
        assert!(
            sh_portable
                .iter()
                .any(|l| l.starts_with("export BBB_WEB_OUT_DIR=")
                    && l.contains("bbb-web-dist-lane-1"))
        );
        assert!(!sh_portable.iter().any(|l| l.contains("BBB_VANILLA_ROOT")));

        let pwsh_full = format_shell_exports(
            ShellKind::PowerShell,
            Some(wt),
            "alpha",
            target,
            Some(vanilla),
        );
        assert!(pwsh_full
            .iter()
            .any(|l| l.starts_with("Set-Location -LiteralPath ")));
        assert!(pwsh_full
            .iter()
            .any(|l| l == "$env:BBB_CARGO_TARGET_NAME = 'alpha'"));
        assert!(pwsh_full
            .iter()
            .any(|l| l.starts_with("$env:BBB_WASM_TARGET_DIR = ")
                && l.contains("bbb-target-wasm-lane-1")));
        assert!(
            pwsh_full
                .iter()
                .any(|l| l.starts_with("$env:BBB_WEB_OUT_DIR = ")
                    && l.contains("bbb-web-dist-lane-1"))
        );
        assert!(pwsh_full
            .iter()
            .any(|l| l.starts_with("$env:BBB_VANILLA_ROOT = ")));

        let cmd_portable = format_shell_exports(ShellKind::Cmd, Some(wt), "alpha", target, None);
        assert!(cmd_portable.iter().any(|l| l.starts_with("cd /d \"")));
        assert!(cmd_portable
            .iter()
            .any(|l| l == "set \"BBB_CARGO_TARGET_NAME=alpha\""));
        assert!(cmd_portable
            .iter()
            .any(|l| l.starts_with("set \"BBB_WASM_TARGET_DIR=")
                && l.contains("bbb-target-wasm-lane-1")));
        assert!(cmd_portable
            .iter()
            .any(|l| l.starts_with("set \"BBB_WEB_OUT_DIR=") && l.contains("bbb-web-dist-lane-1")));
        assert!(!cmd_portable.iter().any(|l| l.contains("BBB_VANILLA_ROOT")));
    }

    #[test]
    fn lease_lock_and_stale_worktree_gc_reclaim_abandoned_slots() {
        let tmp = env::temp_dir().join(format!("bbb-xtask-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let repo = tmp.join("bbb");
        let alive_wt = tmp.join("bbb-wt-alive");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&alive_wt).unwrap();

        let lease_file = repo.join("target-leases.tsv");
        {
            let _guard = LeaseFileLock::acquire(&lease_file).unwrap();
            assert!(lease_file.with_extension("lock").is_file());
        }
        assert!(!lease_file.with_extension("lock").exists());

        let mut leases = vec![
            (1, "stale_deleted".to_string()),
            (2, "alive".to_string()),
            (3, "requesting".to_string()),
        ];
        assert!(gc_stale_leases(&repo, &mut leases, Some("requesting")));
        assert_eq!(
            leases,
            vec![(2, "alive".to_string()), (3, "requesting".to_string())]
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn gate_steps_skip_vanilla_drops_vanilla_only_steps() {
        let full = gate::gate_steps(false);
        let portable = gate::gate_steps(true);
        assert_eq!(full.len(), 14);
        assert_eq!(portable.len(), 12);
        let portable_keys: Vec<&str> = portable.iter().map(|(k, _)| *k).collect();
        assert!(!portable_keys.contains(&"vanilla-verify"));
        assert!(!portable_keys.contains(&"anchors"));
        assert!(!portable_keys.contains(&"check"));
        assert!(portable_keys.contains(&"docs-lint"));
        assert!(portable_keys.contains(&"scripts-tests"));
        assert!(portable_keys.contains(&"clippy"));
    }

    #[test]
    fn build_web_and_web_up_help_flags_return_none_without_error() {
        fn noop() {}
        assert!(parse_build_web_args(&["--help".to_string()], noop)
            .unwrap()
            .is_none());
        assert!(parse_web_up_args(&["--help".to_string()], noop)
            .unwrap()
            .is_none());
    }

    #[test]
    fn target_dir_for_arg_accepts_full_path_even_when_root_has_trailing_separator() {
        let root_with_sep = if cfg!(windows) {
            PathBuf::from(r"C:\Temp\")
        } else {
            PathBuf::from("/custom-root/")
        };
        let full_path = root_with_sep.join("bbb-target-foo");
        assert_eq!(
            target_dir_for_arg_with_root(&full_path.to_string_lossy(), &root_with_sep).unwrap(),
            full_path
        );
        assert_eq!(
            target_dir_for_arg_with_root("foo", &root_with_sep).unwrap(),
            full_path
        );
        assert_eq!(
            target_dir_for_arg_with_root("/tmp/bbb-target-bar", &root_with_sep).unwrap(),
            PathBuf::from("/tmp/bbb-target-bar")
        );
        assert!(target_dir_for_arg_with_root("", &root_with_sep).is_err());
        assert!(target_dir_for_arg_with_root("../escape", &root_with_sep).is_err());
        let outside = PathBuf::from(if cfg!(windows) {
            r"C:\Other\bbb-target-foo"
        } else {
            "/other-root/bbb-target-foo"
        });
        assert!(target_dir_for_arg_with_root(&outside.to_string_lossy(), &root_with_sep).is_err());
    }

    #[test]
    fn cache_and_batch_gate_help_flags_succeed() {
        let help = vec!["--help".to_string()];
        let short_help = vec!["-h".to_string()];
        let root = repo_root();
        assert_eq!(run_size(&help).unwrap(), ExitCode::SUCCESS);
        assert_eq!(run_clean_target(&help).unwrap(), ExitCode::SUCCESS);
        assert_eq!(run_prune(&root, &help).unwrap(), ExitCode::SUCCESS);
        assert_eq!(
            run_batch_gate(&root, &short_help).unwrap(),
            ExitCode::SUCCESS
        );
    }
}
