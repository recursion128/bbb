//! Full gate, lint/target checks, and git hooks (`cargo dev gate|lint|check-targets|install-hooks`).

use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::Instant,
};

use crate::{
    bench::check_feature_gated_benches,
    toolchain::{
        cargo_command, git_common_dir, maybe_prepared_vanilla_root, path_with_toolchain_bin,
        prepare_python_command, resolve_cargo_bin, resolve_target_dir, run_python_tool,
        status_to_exit_code, utc_iso8601_now, XtaskError, MIN_TEST_STACK_BYTES,
    },
};

pub const GATE_STEPS: &[(&str, &str)] = &[
    ("vanilla-verify", "Vanilla workspace verify"),
    ("host-doctor", "portable host capabilities"),
    ("docs-lint", "docs-lint"),
    ("fmt", "cargo fmt --all --check"),
    ("anchors", "Vanilla anchors"),
    ("p0", "P0 audit"),
    ("scripts-lint", "scripts python lint"),
    ("scripts-tests", "scripts python tests"),
    ("diff-check", "git diff --check"),
    ("clippy", "cargo clippy --workspace --all-targets"),
    ("wasm-check", "wasm32-unknown-unknown check"),
    ("cross-check", "second-platform cross-check"),
    ("test", "cargo test --workspace"),
    ("doctest", "cargo test --doc --workspace"),
];

pub fn gate_steps(skip_vanilla: bool) -> Vec<(&'static str, &'static str)> {
    if !skip_vanilla {
        return GATE_STEPS.to_vec();
    }
    GATE_STEPS
        .iter()
        .copied()
        .filter(|(key, _)| !matches!(*key, "vanilla-verify" | "anchors"))
        .map(|(key, label)| match key {
            "docs-lint" => (key, "docs-lint (--skip-vanilla)"),
            "scripts-tests" => (key, "scripts python tests (portable)"),
            _ => (key, label),
        })
        .collect()
}

fn git_head(root: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let head = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!head.is_empty()).then_some(head)
}

fn git_is_clean(root: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .map(|o| o.status.success() && o.stdout.is_empty())
        .unwrap_or(false)
}

pub fn record_gate_stamp(
    root: &Path,
    log_basename: &str,
    pointer_basename: Option<&str>,
    tree_state: &str,
    detail: &str,
    validated_head: Option<&str>,
) -> Result<(), XtaskError> {
    let Some(common_dir) = git_common_dir(root) else {
        return Ok(());
    };
    let head_owned;
    let head = match validated_head.filter(|h| !h.is_empty()) {
        Some(h) => h,
        None => {
            let Some(cur) = git_head(root) else {
                return Ok(());
            };
            head_owned = cur;
            &head_owned
        }
    };

    let stamp_dir = common_dir.join("bbb");
    fs::create_dir_all(&stamp_dir)
        .map_err(|err| XtaskError::failed(format!("create {}: {err}", stamp_dir.display())))?;
    let log_path = stamp_dir.join(log_basename);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| XtaskError::failed(format!("open {}: {err}", log_path.display())))?;
    writeln!(
        file,
        "{}\t{head}\t{tree_state}\t{detail}",
        utc_iso8601_now()
    )
    .map_err(|err| XtaskError::failed(format!("write {}: {err}", log_path.display())))?;

    if let Some(pointer) = pointer_basename.filter(|p| !p.is_empty()) {
        let ptr_path = stamp_dir.join(pointer);
        fs::write(&ptr_path, format!("{head}\n"))
            .map_err(|err| XtaskError::failed(format!("write {}: {err}", ptr_path.display())))?;
    }
    Ok(())
}

pub(crate) fn shell_command(snippet: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("cmd");
        c.raw_arg(format!("/d /s /c \"{snippet}\""));
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.args(["-c", snippet]);
        c
    }
}

fn run_shell_test_command(root: &Path, snippet: &str) -> Result<i32, XtaskError> {
    let mut cmd = shell_command(snippet);
    cmd.current_dir(root)
        .env_remove("BBB_GATE_LOCALIZATION_ONLY");
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("run test command: {err}")))?;
    Ok(status.code().unwrap_or(1))
}

struct GateRunner {
    total: usize,
    index: usize,
    timings: String,
}

impl GateRunner {
    fn new(total: usize) -> Self {
        Self {
            total,
            index: 0,
            timings: String::new(),
        }
    }

    fn step<F>(&mut self, key: &str, label: &str, f: F) -> Result<(), XtaskError>
    where
        F: FnOnce() -> Result<i32, XtaskError>,
    {
        self.index += 1;
        println!("== [{}/{}] {label} ==", self.index, self.total);
        let start = Instant::now();
        let code = f()?;
        if code == 0 {
            let secs = start.elapsed().as_secs();
            println!("   {label}: {secs}s");
            self.timings.push_str(&format!(",{key}:{secs}s"));
            Ok(())
        } else {
            eprintln!("gate step failed: {label} (exit {code})");
            Err(XtaskError::with_code(u8::try_from(code).unwrap_or(1), ""))
        }
    }
}

fn resolve_gate_vanilla_root(root: &Path) -> Result<PathBuf, XtaskError> {
    if let Some(prepared) = maybe_prepared_vanilla_root() {
        return Ok(prepared);
    }
    let out = prepare_python_command(
        root,
        &[
            "python".to_string(),
            "-m".to_string(),
            "bbb_tools.vanilla.cli".to_string(),
            "path".to_string(),
            "26.1".to_string(),
            "--kind".to_string(),
            "root".to_string(),
        ],
    )?
    .output()
    .map_err(|err| XtaskError::failed(format!("resolve vanilla path: {err}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(XtaskError::with_code(
            u8::try_from(out.status.code().unwrap_or(1)).unwrap_or(1),
            stderr.trim().to_string(),
        ));
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

pub fn run_gate(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut dry_run = false;
    let mut skip_vanilla = false;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!(
                    "Usage: cargo dev gate [--dry-run] [--skip-vanilla]\n\n\
                     Runs all {} full-gate steps sequentially and records <git-common-dir>/bbb/last-gated.\n\
                     Pass --skip-vanilla for portable mode (skips Vanilla verify/anchors and Vanilla-required\n\
                     checks; records mode:portable in gate-log.tsv without advancing last-gated).",
                    GATE_STEPS.len()
                );
                return Ok(ExitCode::SUCCESS);
            }
            "--dry-run" => dry_run = true,
            "--skip-vanilla" => skip_vanilla = true,
            other => {
                return Err(XtaskError::usage(format!(
                    "unknown gate option {other:?} (expected --dry-run or --skip-vanilla)"
                )));
            }
        }
    }

    let steps = gate_steps(skip_vanilla);
    if dry_run {
        for (idx, (key, label)) in steps.iter().enumerate() {
            println!("{}/{}\t{key}\t{label}", idx + 1, steps.len());
        }
        return Ok(ExitCode::SUCCESS);
    }

    let localization_only = env::var("BBB_GATE_LOCALIZATION_ONLY").as_deref() == Ok("1");
    let test_command = env::var("BBB_GATE_TEST_COMMAND")
        .ok()
        .filter(|s| !s.is_empty());
    if test_command.is_some() && env::var("BBB_GATE_TEST_MODE").as_deref() != Ok("1") {
        return Err(XtaskError::usage(
            "BBB_GATE_TEST_COMMAND is reserved for isolated script tests",
        ));
    }

    let validated_head = git_head(root);
    let started_clean = validated_head.is_some() && git_is_clean(root);
    let gate_start = Instant::now();

    let mut runner = if let Some(cmd_str) = test_command {
        let mut r = GateRunner::new(1);
        r.step("test-command", "gate test command", || {
            run_shell_test_command(root, &cmd_str)
        })?;
        r
    } else {
        let cargo_bin = resolve_cargo_bin()?;
        let target_dir = resolve_target_dir()?;
        let vanilla_root = if skip_vanilla {
            None
        } else {
            Some(resolve_gate_vanilla_root(root)?)
        };

        let run_py = |py_args: &[&str]| -> Result<i32, XtaskError> {
            let owned: Vec<String> = py_args.iter().map(|s| (*s).to_string()).collect();
            let mut cmd = prepare_python_command(root, &owned)?;
            if let Some(vr) = &vanilla_root {
                cmd.env("BBB_VANILLA_ROOT", vr);
            } else {
                cmd.env_remove("BBB_VANILLA_ROOT");
            }
            cmd.env("CARGO", &cargo_bin)
                .env("PATH", path_with_toolchain_bin(&cargo_bin))
                .env("CARGO_TARGET_DIR", &target_dir)
                .env_remove("BBB_GATE_LOCALIZATION_ONLY");
            let status = cmd
                .status()
                .map_err(|err| XtaskError::failed(format!("python gate step: {err}")))?;
            Ok(status.code().unwrap_or(1))
        };

        let run_cargo_step = |cargo_args: &[&str]| -> Result<i32, XtaskError> {
            let mut cmd = cargo_command(root, &cargo_bin, &target_dir);
            if let Some(vr) = &vanilla_root {
                cmd.env("BBB_VANILLA_ROOT", vr);
            } else {
                cmd.env_remove("BBB_VANILLA_ROOT");
            }
            cmd.env_remove("BBB_GATE_LOCALIZATION_ONLY")
                .args(cargo_args);
            let status = cmd
                .status()
                .map_err(|err| XtaskError::failed(format!("cargo gate step: {err}")))?;
            Ok(status.code().unwrap_or(1))
        };

        let run_cargo_test_step = |cargo_args: &[&str]| -> Result<i32, XtaskError> {
            let mut cmd = cargo_command(root, &cargo_bin, &target_dir);
            if env::var_os("RUST_MIN_STACK").is_none() {
                cmd.env("RUST_MIN_STACK", MIN_TEST_STACK_BYTES);
            }
            if let Some(vr) = &vanilla_root {
                cmd.env("BBB_VANILLA_ROOT", vr)
                    .env("BBB_REQUIRE_VANILLA", "1");
            } else {
                cmd.env_remove("BBB_VANILLA_ROOT")
                    .env_remove("BBB_REQUIRE_VANILLA");
            }
            cmd.env_remove("BBB_GATE_LOCALIZATION_ONLY")
                .args(cargo_args);
            let status = cmd
                .status()
                .map_err(|err| XtaskError::failed(format!("cargo test step: {err}")))?;
            Ok(status.code().unwrap_or(1))
        };

        let mut r = GateRunner::new(steps.len());
        if !skip_vanilla {
            r.step("vanilla-verify", "Vanilla workspace verify", || {
                run_py(&["python", "-m", "bbb_tools.vanilla.cli", "verify", "26.1"])
            })?;
        }
        r.step("host-doctor", "portable host capabilities", || {
            run_py(&[
                "python",
                "-m",
                "bbb_tools.dev_cli",
                "doctor",
                "--profile",
                "portable",
            ])
        })?;
        if skip_vanilla {
            r.step("docs-lint", "docs-lint (--skip-vanilla)", || {
                run_py(&["python", "scripts/docs_lint.py", "--skip-vanilla"])
            })?;
        } else {
            r.step("docs-lint", "docs-lint", || {
                run_py(&["python", "scripts/docs_lint.py", "--all-generators"])
            })?;
        }
        r.step("fmt", "cargo fmt --all --check", || {
            run_cargo_step(&["fmt", "--all", "--check"])
        })?;
        if !skip_vanilla {
            r.step("anchors", "Vanilla anchors", || {
                run_py(&["python", "scripts/check_vanilla_anchors.py", "--all"])
            })?;
        }
        r.step("p0", "P0 audit", || {
            run_py(&["python", "scripts/audit_p0.py"])
        })?;
        r.step("scripts-lint", "scripts python lint", || {
            run_py(&["python", "-m", "bbb_tools.dev_cli", "lint", "python"])
        })?;
        if skip_vanilla {
            r.step("scripts-tests", "scripts python tests (portable)", || {
                run_py(&[
                    "python",
                    "-m",
                    "bbb_tools.dev_cli",
                    "test",
                    "python",
                    "--suite",
                    "portable",
                ])
            })?;
        } else {
            r.step("scripts-tests", "scripts python tests", || {
                run_py(&[
                    "python",
                    "-m",
                    "bbb_tools.dev_cli",
                    "test",
                    "python",
                    "--suite",
                    "all",
                ])
            })?;
        }
        r.step("diff-check", "git diff --check", || {
            let status = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["diff", "--check"])
                .status()
                .map_err(|err| XtaskError::failed(format!("git diff --check: {err}")))?;
            Ok(status.code().unwrap_or(1))
        })?;
        // `cargo clippy --workspace --all-targets` already type-checks every
        // library, binary, test, bench, and example target; also check any
        // extra feature-gated Criterion benches if present.
        r.step("clippy", "cargo clippy --workspace --all-targets", || {
            let code = run_cargo_clippy(root, &cargo_bin, &target_dir, vanilla_root.as_deref())?;
            if code != 0 {
                return Ok(code);
            }
            check_feature_gated_benches(root, &cargo_bin, &target_dir, vanilla_root.as_deref())
        })?;
        r.step("wasm-check", "wasm32-unknown-unknown check", || {
            crate::web::check_web(root)
        })?;
        r.step("cross-check", "second-platform cross-check", || {
            run_py(&["python", "scripts/cross_check.py"])
        })?;
        r.step("test", "cargo test --workspace", || {
            run_cargo_test_step(&[
                "test",
                "--locked",
                "--workspace",
                "--no-fail-fast",
                "--lib",
                "--bins",
                "--tests",
            ])
        })?;
        r.step("doctest", "cargo test --doc --workspace", || {
            run_cargo_test_step(&["test", "--locked", "--doc", "--workspace"])
        })?;
        r
    };

    let total_secs = gate_start.elapsed().as_secs();
    let current_head = git_head(root);
    let clean_same_head = started_clean && current_head == validated_head && git_is_clean(root);
    let (tree_state, pointer) = if clean_same_head && !skip_vanilla {
        ("clean", Some("last-gated"))
    } else if clean_same_head {
        ("clean", None)
    } else {
        ("dirty", None)
    };
    let mode = if skip_vanilla { "portable" } else { "full" };
    let detail = format!("mode:{mode},total:{total_secs}s{}", runner.timings);
    if localization_only {
        println!("gate: localization-only run left formal gate state unchanged");
    } else {
        record_gate_stamp(
            root,
            "gate-log.tsv",
            pointer,
            tree_state,
            &detail,
            validated_head.as_deref(),
        )?;
    }
    println!("gate: all {} steps passed in {total_secs}s", runner.total);
    let _ = &mut runner;
    Ok(ExitCode::SUCCESS)
}

fn run_cargo_clippy(
    root: &Path,
    cargo_bin: &Path,
    target_dir: &Path,
    vanilla_root: Option<&Path>,
) -> Result<i32, XtaskError> {
    let mut cmd = cargo_command(root, cargo_bin, target_dir);
    if let Some(vr) = vanilla_root {
        cmd.env("BBB_VANILLA_ROOT", vr);
    } else {
        cmd.env_remove("BBB_VANILLA_ROOT");
    }
    cmd.env_remove("BBB_GATE_LOCALIZATION_ONLY").args([
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ]);
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("cargo clippy: {err}")))?;
    Ok(status.code().unwrap_or(1))
}

pub fn run_lint(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut skip_vanilla = false;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!(
                    "Usage: cargo dev lint [--skip-vanilla]\n\n\
                     Runs repository lints: cargo fmt --check, git diff --check,\n\
                     audit-p0, py-lint, docs-lint, (unless --skip-vanilla) anchors,\n\
                     and cargo clippy --workspace --all-targets -- -D warnings."
                );
                return Ok(ExitCode::SUCCESS);
            }
            "--skip-vanilla" => skip_vanilla = true,
            other => {
                return Err(XtaskError::usage(format!(
                    "unknown lint flag {other:?} (expected --skip-vanilla)"
                )));
            }
        }
    }

    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let vanilla_root = if skip_vanilla {
        None
    } else {
        Some(resolve_gate_vanilla_root(root)?)
    };

    let run_py = |py_args: &[&str]| -> Result<i32, XtaskError> {
        let owned: Vec<String> = py_args.iter().map(|s| (*s).to_string()).collect();
        let mut cmd = prepare_python_command(root, &owned)?;
        if let Some(vr) = &vanilla_root {
            cmd.env("BBB_VANILLA_ROOT", vr);
        }
        cmd.env("CARGO", &cargo_bin)
            .env("CARGO_TARGET_DIR", &target_dir);
        let status = cmd
            .status()
            .map_err(|err| XtaskError::failed(format!("python lint step: {err}")))?;
        Ok(status.code().unwrap_or(1))
    };

    let total = if skip_vanilla { 6 } else { 7 };
    let mut r = GateRunner::new(total);
    r.step("fmt", "cargo fmt --all --check", || {
        let status = cargo_command(root, &cargo_bin, &target_dir)
            .args(["fmt", "--all", "--check"])
            .status()
            .map_err(|err| XtaskError::failed(format!("cargo fmt: {err}")))?;
        Ok(status.code().unwrap_or(1))
    })?;
    r.step("diff-check", "git diff --check", || {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["diff", "--check"])
            .status()
            .map_err(|err| XtaskError::failed(format!("git diff --check: {err}")))?;
        Ok(status.code().unwrap_or(1))
    })?;
    r.step("p0", "P0 audit", || {
        run_py(&["python", "scripts/audit_p0.py"])
    })?;
    r.step("scripts-lint", "scripts python lint", || {
        run_py(&["python", "-m", "bbb_tools.dev_cli", "lint", "python"])
    })?;
    if skip_vanilla {
        r.step("docs-lint", "docs-lint (--skip-vanilla)", || {
            run_py(&["python", "scripts/docs_lint.py", "--skip-vanilla"])
        })?;
    } else {
        r.step("docs-lint", "docs-lint", || {
            run_py(&["python", "scripts/docs_lint.py"])
        })?;
        r.step("anchors", "Vanilla anchors", || {
            run_py(&["python", "scripts/check_vanilla_anchors.py"])
        })?;
    }
    r.step("clippy", "cargo clippy --workspace --all-targets", || {
        run_cargo_clippy(root, &cargo_bin, &target_dir, vanilla_root.as_deref())
    })?;
    println!("lint: all {} steps passed", r.total);
    Ok(ExitCode::SUCCESS)
}

pub fn run_check_targets(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    if let Some(arg) = args.first() {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!(
                    "Usage: cargo dev check-targets\n\n\
                     Type-checks host workspace (--all-targets + feature-gated benches),\n\
                     wasm32-unknown-unknown Web client, and second-platform desktop crates (cross-check)."
                );
                return Ok(ExitCode::SUCCESS);
            }
            other => {
                return Err(XtaskError::usage(format!(
                    "unknown check-targets flag {other:?}"
                )));
            }
        }
    }

    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let vanilla = maybe_prepared_vanilla_root();
    let mut r = GateRunner::new(3);
    r.step("check", "cargo check --workspace --all-targets", || {
        let status = cargo_command(root, &cargo_bin, &target_dir)
            .args(["check", "--locked", "--workspace", "--all-targets"])
            .status()
            .map_err(|err| XtaskError::failed(format!("cargo check: {err}")))?;
        let code = status.code().unwrap_or(1);
        if code != 0 {
            return Ok(code);
        }
        check_feature_gated_benches(root, &cargo_bin, &target_dir, vanilla.as_deref())
    })?;
    r.step("wasm-check", "wasm32-unknown-unknown check", || {
        crate::web::check_web(root)
    })?;
    r.step("cross-check", "second-platform cross-check", || {
        let owned = vec!["python".to_string(), "scripts/cross_check.py".to_string()];
        let status = prepare_python_command(root, &owned)?
            .env("CARGO", &cargo_bin)
            .env("PATH", path_with_toolchain_bin(&cargo_bin))
            .env("CARGO_TARGET_DIR", &target_dir)
            .status()
            .map_err(|err| XtaskError::failed(format!("cross-check: {err}")))?;
        Ok(status.code().unwrap_or(1))
    })?;
    println!("check-targets: all 3 target checks passed");
    Ok(ExitCode::SUCCESS)
}

fn pre_commit_docs_lint_args(has_vanilla: bool, require_vanilla: bool) -> Vec<String> {
    let mut args = vec!["python".to_string(), "scripts/docs_lint.py".to_string()];
    if !has_vanilla && !require_vanilla {
        args.push("--skip-vanilla".to_string());
    }
    args
}

fn run_pre_commit_diff_check(root: &Path) -> Result<ExitCode, XtaskError> {
    for args in [
        &["diff", "--cached", "--check"][..],
        &["diff", "--check"][..],
    ] {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .map_err(|err| XtaskError::failed(format!("git {}: {err}", args.join(" "))))?;
        if !status.success() {
            return Ok(status_to_exit_code(status));
        }
    }
    Ok(ExitCode::SUCCESS)
}

pub fn run_pre_commit(root: &Path) -> Result<ExitCode, XtaskError> {
    let cargo_bin = resolve_cargo_bin()?;
    let target_dir = resolve_target_dir()?;
    let fmt_status = cargo_command(root, &cargo_bin, &target_dir)
        .args(["fmt", "--all", "--check"])
        .status()
        .map_err(|err| XtaskError::failed(format!("cargo fmt: {err}")))?;
    if !fmt_status.success() {
        return Ok(status_to_exit_code(fmt_status));
    }

    let diff_code = run_pre_commit_diff_check(root)?;
    if diff_code != ExitCode::SUCCESS {
        return Ok(diff_code);
    }

    let has_vanilla = maybe_prepared_vanilla_root().is_some();
    let require_vanilla = env::var("BBB_REQUIRE_VANILLA").as_deref() == Ok("1");
    let args = pre_commit_docs_lint_args(has_vanilla, require_vanilla);
    run_python_tool(root, &args)
}

pub fn run_install_hooks(root: &Path) -> Result<ExitCode, XtaskError> {
    let common = git_common_dir(root)
        .ok_or_else(|| XtaskError::usage("install-hooks must run inside a Git worktree"))?;
    let hooks_dir = common.join("hooks");
    fs::create_dir_all(&hooks_dir)
        .map_err(|err| XtaskError::failed(format!("create {}: {err}", hooks_dir.display())))?;
    let dst = hooks_dir.join("pre-commit");
    let script = "\
#!/bin/sh
set -eu
if ! command -v cargo >/dev/null 2>&1; then
  for dir in \"${CARGO_HOME:-}/bin\" \"${HOME:-}/.cargo/bin\" \"${USERPROFILE:-}/.cargo/bin\"; do
    if [ -n \"$dir\" ] && [ -d \"$dir\" ]; then
      PATH=\"$dir:$PATH\"
    fi
  done
  export PATH
fi
cargo dev pre-commit
";
    fs::write(&dst, script)
        .map_err(|err| XtaskError::failed(format!("write {}: {err}", dst.display())))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dst)
            .map_err(|err| XtaskError::failed(format!("stat {}: {err}", dst.display())))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dst, perms)
            .map_err(|err| XtaskError::failed(format!("chmod {}: {err}", dst.display())))?;
    }
    println!("installed {}", dst.display());
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_commit_skips_vanilla_only_when_cache_is_absent_and_not_required() {
        assert_eq!(
            pre_commit_docs_lint_args(false, false),
            vec!["python", "scripts/docs_lint.py", "--skip-vanilla"]
        );
        assert_eq!(
            pre_commit_docs_lint_args(false, true),
            vec!["python", "scripts/docs_lint.py"]
        );
        assert_eq!(
            pre_commit_docs_lint_args(true, false),
            vec!["python", "scripts/docs_lint.py"]
        );
        assert_eq!(
            pre_commit_docs_lint_args(true, true),
            vec!["python", "scripts/docs_lint.py"]
        );
    }

    #[test]
    fn pre_commit_diff_check_rejects_staged_trailing_whitespace() {
        let tmp = env::temp_dir().join(format!("bbb-xtask-pre-commit-diff-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let run_git = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&tmp)
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };

        run_git(&["init", "-q"]);
        run_git(&["config", "user.name", "Gate Test"]);
        run_git(&["config", "user.email", "gate-test@bbb.invalid"]);
        fs::write(tmp.join("file.txt"), "clean\n").unwrap();
        run_git(&["add", "file.txt"]);
        run_git(&["commit", "-qm", "initial"]);

        // Stage trailing whitespace so unstaged `git diff --check` is empty.
        fs::write(tmp.join("file.txt"), "dirty   \n").unwrap();
        run_git(&["add", "file.txt"]);
        assert_ne!(run_pre_commit_diff_check(&tmp).unwrap(), ExitCode::SUCCESS);

        fs::write(tmp.join("file.txt"), "clean again\n").unwrap();
        run_git(&["add", "file.txt"]);
        assert_eq!(run_pre_commit_diff_check(&tmp).unwrap(), ExitCode::SUCCESS);

        let _ = fs::remove_dir_all(&tmp);
    }
}
