//! Isolated worker worktrees and warm Cargo target-pool leases (`cargo worker`).

use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    thread,
    time::{Duration, SystemTime},
};

use crate::toolchain::{
    default_target_root, git_common_dir, maybe_prepared_vanilla_root, normalize_git_path,
    prepare_python_command, primary_worktree_root, status_to_exit_code, utc_compact_timestamp,
    venv_python_path, XtaskError,
};

const LEASE_LOCK_STALE_SECS: u64 = 60;
const LEASE_LOCK_RETRY_MS: u64 = 50;
const LEASE_LOCK_MAX_ATTEMPTS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Sh,
    PowerShell,
    Cmd,
}

impl ShellKind {
    pub fn default_for_host() -> Self {
        if cfg!(windows) {
            Self::PowerShell
        } else {
            Self::Sh
        }
    }

    pub fn parse(value: &str) -> Result<Self, XtaskError> {
        match value {
            "sh" | "posix" | "bash" | "zsh" => Ok(Self::Sh),
            "powershell" | "pwsh" => Ok(Self::PowerShell),
            "cmd" | "bat" => Ok(Self::Cmd),
            other => Err(XtaskError::usage(format!(
                "unknown --shell {other:?} (expected sh, powershell, or cmd)"
            ))),
        }
    }
}

pub fn print_worker_usage() {
    eprintln!(
        "\
Usage:
  cargo worker create <name> [base-ref]
  cargo worker run <name> [--] <command> [args...]
  cargo worker status [name]
  cargo worker cleanup <name> [--force|-f]
  cargo worker release <name>
  cargo worker env <name> [--shell <sh|powershell|cmd>]
  cargo worker shell-env <name> [--shell <sh|powershell|cmd>]

Conventions:
  worktree path: ../bbb-wt-<name>
  branch:        worker/<name>
  target dir:    leased warm-pool slot <temp>/bbb-target-lane-<N> (N in 1..=BBB_TARGET_POOL_SIZE, default 8)
                 recorded in <git-common-dir>/bbb/target-leases.tsv"
    );
}

pub fn is_pool_slot(name: &str) -> bool {
    let Some(digits) = name.strip_prefix("lane-") else {
        return false;
    };
    (1..=3).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
}

fn target_pool_size() -> usize {
    env::var("BBB_TARGET_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(8)
}

fn target_pool_root() -> PathBuf {
    env::var_os("BBB_TARGET_POOL_ROOT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_target_root)
}

fn target_pool_slot_dir(slot: usize) -> PathBuf {
    target_pool_root().join(format!("bbb-target-lane-{slot}"))
}

pub fn worker_wasm_target_dir(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    if let Some(suffix) = target
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("bbb-target-"))
    {
        parent.join(format!("bbb-target-wasm-{suffix}"))
    } else {
        target.join("wasm")
    }
}

pub fn worker_web_out_dir(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    if let Some(suffix) = target
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("bbb-target-"))
    {
        parent.join(format!("bbb-web-dist-{suffix}"))
    } else {
        target.join("web-dist")
    }
}

fn target_pool_lease_file(root: &Path) -> Result<PathBuf, XtaskError> {
    let common = git_common_dir(root)
        .ok_or_else(|| XtaskError::usage("worker commands must run inside a Git worktree"))?;
    Ok(common.join("bbb").join("target-leases.tsv"))
}

pub(crate) struct LeaseFileLock {
    lock_path: PathBuf,
}

impl LeaseFileLock {
    pub(crate) fn acquire(lease_file: &Path) -> Result<Self, XtaskError> {
        if let Some(parent) = lease_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| XtaskError::failed(format!("create {}: {err}", parent.display())))?;
        }
        let lock_path = lease_file.with_extension("lock");
        for _ in 0..LEASE_LOCK_MAX_ATTEMPTS {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    let _ = writeln!(file, "pid={}", std::process::id());
                    return Ok(Self { lock_path });
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    let is_stale = fs::metadata(&lock_path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|mtime| SystemTime::now().duration_since(mtime).ok())
                        .is_some_and(|age| age.as_secs() >= LEASE_LOCK_STALE_SECS);
                    if is_stale {
                        let _ = fs::remove_file(&lock_path);
                        continue;
                    }
                    thread::sleep(Duration::from_millis(LEASE_LOCK_RETRY_MS));
                }
                Err(err) => {
                    return Err(XtaskError::failed(format!(
                        "acquire lease lock {}: {err}",
                        lock_path.display()
                    )));
                }
            }
        }
        Err(XtaskError::failed(format!(
            "timed out waiting for target lease lock {}",
            lock_path.display()
        )))
    }
}

impl Drop for LeaseFileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

fn read_leases(lease_file: &Path) -> Vec<(usize, String)> {
    let Ok(text) = fs::read_to_string(lease_file) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let (slot_str, lane) = line.split_once('\t')?;
            let slot = slot_str.trim().parse::<usize>().ok()?;
            Some((slot, lane.trim().to_string()))
        })
        .collect()
}

fn write_leases(lease_file: &Path, leases: &[(usize, String)]) -> Result<(), XtaskError> {
    if let Some(parent) = lease_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| XtaskError::failed(format!("create {}: {err}", parent.display())))?;
    }
    let mut buf = String::new();
    for (slot, lane) in leases {
        buf.push_str(&format!("{slot}\t{lane}\n"));
    }
    let tmp = lease_file.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp, buf)
        .map_err(|err| XtaskError::failed(format!("write {}: {err}", tmp.display())))?;
    fs::rename(&tmp, lease_file)
        .map_err(|err| XtaskError::failed(format!("rename {}: {err}", lease_file.display())))?;
    Ok(())
}

/// Garbage-collect leases whose worker worktree directory no longer exists on disk.
pub(crate) fn gc_stale_leases(
    toplevel: &Path,
    leases: &mut Vec<(usize, String)>,
    keep_lane: Option<&str>,
) -> bool {
    let before = leases.len();
    leases.retain(|(_, owner)| {
        if keep_lane == Some(owner.as_str()) {
            return true;
        }
        worktree_path(toplevel, owner).is_ok_and(|path| path.exists())
    });
    leases.len() != before
}

fn target_pool_lookup(root: &Path, lane: &str) -> Option<PathBuf> {
    let file = target_pool_lease_file(root).ok()?;
    read_leases(&file)
        .into_iter()
        .find(|(_, l)| l == lane)
        .map(|(slot, _)| target_pool_slot_dir(slot))
}

fn target_pool_lease(root: &Path, lane: &str) -> Result<PathBuf, XtaskError> {
    let primary = primary_worktree_root(root).unwrap_or_else(|_| root.to_path_buf());
    let file = target_pool_lease_file(&primary)?;
    let _lock = LeaseFileLock::acquire(&file)?;
    let mut leases = read_leases(&file);
    let pruned = gc_stale_leases(&primary, &mut leases, Some(lane));
    if let Some((slot, _)) = leases.iter().find(|(_, l)| l == lane) {
        let slot = *slot;
        if pruned {
            write_leases(&file, &leases)?;
        }
        return Ok(target_pool_slot_dir(slot));
    }
    let max = target_pool_size();
    for slot in 1..=max {
        if !leases.iter().any(|(s, _)| *s == slot) {
            leases.push((slot, lane.to_string()));
            write_leases(&file, &leases)?;
            return Ok(target_pool_slot_dir(slot));
        }
    }
    if pruned {
        write_leases(&file, &leases)?;
    }
    let mut msg = format!(
        "target pool: every slot 1..{max} is leased; release one with cargo worker cleanup/release"
    );
    for (slot, owner) in &leases {
        msg.push_str(&format!("\n  lane-{slot} -> {owner}"));
    }
    Err(XtaskError::with_code(3, msg))
}

fn target_pool_release(root: &Path, lane: &str) -> Result<(), XtaskError> {
    let primary = primary_worktree_root(root).unwrap_or_else(|_| root.to_path_buf());
    let Ok(file) = target_pool_lease_file(&primary) else {
        return Ok(());
    };
    let _lock = LeaseFileLock::acquire(&file)?;
    if !file.is_file() {
        return Ok(());
    }
    let mut leases: Vec<(usize, String)> = read_leases(&file)
        .into_iter()
        .filter(|(_, l)| l != lane)
        .collect();
    gc_stale_leases(&primary, &mut leases, None);
    write_leases(&file, &leases)
}

fn validate_worker_name(name: &str) -> Result<(), XtaskError> {
    if name.is_empty() {
        return Err(XtaskError::usage("worker name must not be empty"));
    }
    if name.starts_with('-') {
        return Err(XtaskError::usage(format!(
            "worker name must not start with '-': {name}"
        )));
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(XtaskError::usage(format!(
            "worker name may contain only ASCII letters, digits, '_' and '-': {name}"
        )));
    }
    Ok(())
}

fn git_worktree_toplevel(root: &Path) -> Result<PathBuf, XtaskError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|err| XtaskError::usage(format!("run git rev-parse: {err}")))?;
    if !out.status.success() {
        return Err(XtaskError::usage(
            "worker commands must run inside a Git worktree",
        ));
    }
    Ok(normalize_git_path(&String::from_utf8_lossy(&out.stdout)))
}

fn worker_primary_root(root: &Path) -> Result<PathBuf, XtaskError> {
    let _ = git_worktree_toplevel(root)?;
    primary_worktree_root(root)
}

pub(crate) fn worktree_path(root: &Path, name: &str) -> Result<PathBuf, XtaskError> {
    let parent = root.parent().ok_or_else(|| {
        XtaskError::usage(format!(
            "repository root has no parent directory: {}",
            root.display()
        ))
    })?;
    Ok(parent.join(format!("bbb-wt-{name}")))
}

fn branch_exists(root: &Path, branch: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn select_branch_name(root: &Path, name: &str) -> String {
    let base = format!("worker/{name}");
    if !branch_exists(root, &base) {
        return base;
    }
    let stamp = utc_compact_timestamp();
    let mut suffix = 1usize;
    loop {
        let candidate = if suffix == 1 {
            format!("{base}-{stamp}")
        } else {
            format!("{base}-{stamp}-{suffix}")
        };
        if !branch_exists(root, &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn is_worker_branch_for_name(name: &str, branch: &str) -> bool {
    let expected = format!("worker/{name}");
    if branch == expected {
        return true;
    }
    branch
        .strip_prefix(&format!("{expected}-"))
        .is_some_and(|rest| rest.bytes().next().is_some_and(|b| b.is_ascii_digit()))
}

fn resolve_worker_vanilla_root(root: &Path) -> Result<Option<PathBuf>, XtaskError> {
    let primary = primary_worktree_root(root)?;
    if venv_python_path(&primary).is_none() {
        return Err(XtaskError::usage(format!(
            "bbb Python environment is not prepared in the primary worktree\n\
             run: cargo dev bootstrap (from {})",
            primary.display()
        )));
    }

    let require_vanilla = env::var("BBB_REQUIRE_VANILLA").as_deref() == Ok("1");

    if let Some(prepared) = maybe_prepared_vanilla_root() {
        return Ok(Some(prepared));
    }

    if !primary.join("scripts").join("bbb_tools").is_dir() {
        if require_vanilla {
            return Err(XtaskError::usage(
                "Vanilla 26.1 workspace is unavailable; run: cargo dev vanilla prepare 26.1",
            ));
        }
        return Ok(None);
    }

    let out = prepare_python_command(
        &primary,
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
    .map_err(|err| XtaskError::usage(format!("resolve Vanilla workspace: {err}")))?;
    if out.status.success() {
        let resolved = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !resolved.is_empty() {
            return Ok(Some(PathBuf::from(resolved)));
        }
    }

    if require_vanilla {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if stderr.is_empty() {
            "Vanilla 26.1 workspace is unavailable; run: cargo dev vanilla prepare 26.1".to_string()
        } else {
            stderr
        };
        return Err(XtaskError::usage(msg));
    }
    Ok(None)
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn pwsh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub fn format_shell_exports(
    shell: ShellKind,
    cd_path: Option<&Path>,
    name: &str,
    target: &Path,
    vanilla: Option<&Path>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let target_str = target.to_string_lossy();
    let wasm_target = worker_wasm_target_dir(target);
    let wasm_target_str = wasm_target.to_string_lossy();
    let web_out = worker_web_out_dir(target);
    let web_out_str = web_out.to_string_lossy();
    match shell {
        ShellKind::Sh => {
            if let Some(p) = cd_path {
                lines.push(format!("cd {}", sh_quote(&p.to_string_lossy())));
            }
            lines.push(format!("export BBB_CARGO_TARGET_NAME={}", sh_quote(name)));
            lines.push(format!("export CARGO_TARGET_DIR={}", sh_quote(&target_str)));
            lines.push(format!(
                "export BBB_WASM_TARGET_DIR={}",
                sh_quote(&wasm_target_str)
            ));
            lines.push(format!("export BBB_WEB_OUT_DIR={}", sh_quote(&web_out_str)));
            if let Some(vr) = vanilla {
                lines.push(format!(
                    "export BBB_VANILLA_ROOT={}",
                    sh_quote(&vr.to_string_lossy())
                ));
            }
        }
        ShellKind::PowerShell => {
            if let Some(p) = cd_path {
                lines.push(format!(
                    "Set-Location -LiteralPath {}",
                    pwsh_quote(&p.to_string_lossy())
                ));
            }
            lines.push(format!("$env:BBB_CARGO_TARGET_NAME = {}", pwsh_quote(name)));
            lines.push(format!(
                "$env:CARGO_TARGET_DIR = {}",
                pwsh_quote(&target_str)
            ));
            lines.push(format!(
                "$env:BBB_WASM_TARGET_DIR = {}",
                pwsh_quote(&wasm_target_str)
            ));
            lines.push(format!(
                "$env:BBB_WEB_OUT_DIR = {}",
                pwsh_quote(&web_out_str)
            ));
            if let Some(vr) = vanilla {
                lines.push(format!(
                    "$env:BBB_VANILLA_ROOT = {}",
                    pwsh_quote(&vr.to_string_lossy())
                ));
            }
        }
        ShellKind::Cmd => {
            if let Some(p) = cd_path {
                lines.push(format!("cd /d \"{}\"", p.display()));
            }
            lines.push(format!("set \"BBB_CARGO_TARGET_NAME={name}\""));
            lines.push(format!("set \"CARGO_TARGET_DIR={target_str}\""));
            lines.push(format!("set \"BBB_WASM_TARGET_DIR={wasm_target_str}\""));
            lines.push(format!("set \"BBB_WEB_OUT_DIR={web_out_str}\""));
            if let Some(vr) = vanilla {
                lines.push(format!("set \"BBB_VANILLA_ROOT={}\"", vr.display()));
            }
        }
    }
    lines
}

fn parse_env_args(rest: &[String], subcmd: &str) -> Result<(String, ShellKind), XtaskError> {
    let mut name: Option<String> = None;
    let mut shell = ShellKind::default_for_host();
    let mut i = 0;
    while i < rest.len() {
        let arg = rest[i].as_str();
        if arg == "--shell" {
            i += 1;
            let val = rest.get(i).ok_or_else(|| {
                XtaskError::usage(format!(
                    "usage: cargo worker {subcmd} <name> [--shell <sh|powershell|cmd>]"
                ))
            })?;
            shell = ShellKind::parse(val)?;
        } else if let Some(val) = arg.strip_prefix("--shell=") {
            shell = ShellKind::parse(val)?;
        } else if arg.starts_with('-') {
            return Err(XtaskError::usage(format!(
                "unknown {subcmd} option: {arg} (expected --shell <sh|powershell|cmd>)"
            )));
        } else if name.is_none() {
            name = Some(rest[i].clone());
        } else {
            return Err(XtaskError::usage(format!(
                "usage: cargo worker {subcmd} <name> [--shell <sh|powershell|cmd>]"
            )));
        }
        i += 1;
    }
    let Some(name) = name else {
        print_worker_usage();
        return Err(XtaskError::usage(format!(
            "usage: cargo worker {subcmd} <name> [--shell <sh|powershell|cmd>]"
        )));
    };
    Ok((name, shell))
}

fn print_worker_summary(
    name: &str,
    path: &Path,
    branch: &str,
    target: &Path,
    vanilla: Option<&Path>,
) {
    let wasm_target = worker_wasm_target_dir(target);
    let web_out = worker_web_out_dir(target);
    println!("worker: {name}");
    println!("worktree: {}", path.display());
    println!("branch: {branch}");
    println!("BBB_CARGO_TARGET_NAME={name}");
    println!("CARGO_TARGET_DIR={}", target.display());
    println!("BBB_WASM_TARGET_DIR={}", wasm_target.display());
    println!("BBB_WEB_OUT_DIR={}", web_out.display());
    if let Some(vr) = vanilla {
        println!("BBB_VANILLA_ROOT={}", vr.display());
    }
}

fn is_git_worktree(path: &Path) -> bool {
    path.exists()
        && Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

fn git_current_branch(path: &Path) -> String {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub fn run_worker_cli(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let Some(subcmd) = args.first().map(String::as_str) else {
        print_worker_usage();
        return Err(XtaskError::usage("missing worker subcommand"));
    };
    let rest = &args[1..];
    match subcmd {
        "-h" | "--help" | "help" => {
            print_worker_usage();
            Ok(ExitCode::SUCCESS)
        }
        "create" => {
            if rest.is_empty() || rest.len() > 2 {
                print_worker_usage();
                return Err(XtaskError::usage(
                    "usage: cargo worker create <name> [base-ref]",
                ));
            }
            let base_ref = rest.get(1).map(String::as_str).unwrap_or("HEAD");
            worker_create(root, &rest[0], base_ref)
        }
        "run" => {
            if rest.is_empty() {
                print_worker_usage();
                return Err(XtaskError::usage(
                    "usage: cargo worker run <name> [--] <command> [args...]",
                ));
            }
            worker_run(root, &rest[0], &rest[1..])
        }
        "status" => {
            if rest.len() > 1 {
                print_worker_usage();
                return Err(XtaskError::usage("usage: cargo worker status [name]"));
            }
            worker_status(root, rest.first().map(String::as_str))
        }
        "cleanup" => {
            if rest.is_empty() || rest.len() > 2 {
                print_worker_usage();
                return Err(XtaskError::usage(
                    "usage: cargo worker cleanup <name> [--force|-f]",
                ));
            }
            worker_cleanup(root, &rest[0], &rest[1..])
        }
        "release" => {
            if rest.len() != 1 {
                print_worker_usage();
                return Err(XtaskError::usage("usage: cargo worker release <name>"));
            }
            worker_release(root, &rest[0])
        }
        "env" => {
            let (name, shell) = parse_env_args(rest, "env")?;
            worker_env(root, &name, shell)
        }
        "shell-env" => {
            let (name, shell) = parse_env_args(rest, "shell-env")?;
            worker_shell_env(root, &name, shell)
        }
        other => {
            print_worker_usage();
            Err(XtaskError::usage(format!(
                "unknown worker command: {other}"
            )))
        }
    }
}

fn worker_create(root: &Path, name: &str, base_ref: &str) -> Result<ExitCode, XtaskError> {
    validate_worker_name(name)?;
    let toplevel = worker_primary_root(root)?;
    let path = worktree_path(&toplevel, name)?;
    if path.exists() {
        return Err(XtaskError::usage(format!(
            "worktree path already exists: {}",
            path.display()
        )));
    }

    let default_branch = format!("worker/{name}");
    let branch = select_branch_name(&toplevel, name);
    if branch != default_branch {
        eprintln!("branch already exists: {default_branch}");
        eprintln!("using temporary branch: {branch}");
    }

    let vanilla_root = resolve_worker_vanilla_root(&toplevel)?;
    let target = target_pool_lease(&toplevel, name)?;
    fs::create_dir_all(&target)
        .map_err(|err| XtaskError::failed(format!("create {}: {err}", target.display())))?;

    let status = Command::new("git")
        .arg("-C")
        .arg(&toplevel)
        .args(["worktree", "add", "-b", &branch])
        .arg(&path)
        .arg(base_ref)
        .status()
        .map_err(|err| XtaskError::failed(format!("git worktree add: {err}")))?;
    if !status.success() {
        let _ = target_pool_release(&toplevel, name);
        return Err(XtaskError::failed(format!(
            "git worktree add exited with {status}"
        )));
    }

    print_worker_summary(name, &path, &branch, &target, vanilla_root.as_deref());
    println!("\nRun focused tests with:");
    println!(
        "  cargo worker run {} -- cargo dev test -p <crate> <filter>",
        sh_quote(name)
    );
    Ok(ExitCode::SUCCESS)
}

fn worker_run(root: &Path, name: &str, mut cmd_args: &[String]) -> Result<ExitCode, XtaskError> {
    validate_worker_name(name)?;
    if cmd_args.first().map(String::as_str) == Some("--") {
        cmd_args = &cmd_args[1..];
    }
    if cmd_args.is_empty() {
        return Err(XtaskError::usage("worker run requires a command"));
    }

    let toplevel = worker_primary_root(root)?;
    let path = worktree_path(&toplevel, name)?;
    if !is_git_worktree(&path) {
        return Err(XtaskError::usage(format!(
            "worker worktree not found: {}",
            path.display()
        )));
    }
    let current_branch = git_current_branch(&path);
    if !is_worker_branch_for_name(name, &current_branch) {
        let actual = if current_branch.is_empty() {
            "detached HEAD"
        } else {
            &current_branch
        };
        return Err(XtaskError::usage(format!(
            "worker worktree is on an unexpected branch: {}\nactual: {actual}",
            path.display()
        )));
    }

    let Some(target) = target_pool_lookup(&toplevel, name) else {
        return Err(XtaskError::usage(format!(
            "worker target lease not found: {name}"
        )));
    };
    if !target.is_dir() {
        return Err(XtaskError::usage(format!(
            "worker target lease is not warm: {}",
            target.display()
        )));
    }

    let vanilla_root = resolve_worker_vanilla_root(&toplevel)?;
    let wasm_target = worker_wasm_target_dir(&target);
    let web_out = worker_web_out_dir(&target);
    let mut cmd = Command::new(&cmd_args[0]);
    cmd.args(&cmd_args[1..])
        .current_dir(&path)
        .env_remove("BBB_REPO_ROOT")
        .env("BBB_CARGO_TARGET_NAME", name)
        .env("CARGO_TARGET_DIR", &target)
        .env("BBB_WASM_TARGET_DIR", &wasm_target)
        .env("BBB_WEB_OUT_DIR", &web_out);
    if let Some(vr) = &vanilla_root {
        cmd.env("BBB_VANILLA_ROOT", vr);
    } else {
        cmd.env_remove("BBB_VANILLA_ROOT");
    }
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("spawn {}: {err}", cmd_args[0])))?;
    Ok(status_to_exit_code(status))
}

fn print_status_one(toplevel: &Path, name: &str) -> Result<(), XtaskError> {
    validate_worker_name(name)?;
    let path = worktree_path(toplevel, name)?;
    let target_status = match target_pool_lookup(toplevel, name) {
        Some(t) if t.exists() => format!("target={}(warm)", t.display()),
        Some(t) => format!("target={}(cold)", t.display()),
        None => "target=unleased".to_string(),
    };

    if !is_git_worktree(&path) {
        let kind = if path.exists() { "not-git" } else { "missing" };
        println!(
            "{name}: status={kind} path={} {target_status}",
            path.display()
        );
        return Ok(());
    }

    let head = Command::new("git")
        .arg("-C")
        .arg(&path)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let mut branch = git_current_branch(&path);
    if branch.is_empty() {
        branch = "detached".to_string();
    }
    let dirty_lines = Command::new("git")
        .arg("-C")
        .arg(&path)
        .args(["status", "--short"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);
    let state = if dirty_lines > 0 {
        format!("dirty({dirty_lines})")
    } else {
        "clean".to_string()
    };

    println!(
        "{name}: branch={branch} status={state} head={head} path={} {target_status}",
        path.display()
    );
    Ok(())
}

fn worker_status(root: &Path, name: Option<&str>) -> Result<ExitCode, XtaskError> {
    let toplevel = worker_primary_root(root)?;
    if let Some(single) = name {
        print_status_one(&toplevel, single)?;
        return Ok(ExitCode::SUCCESS);
    }

    let parent = toplevel.parent().unwrap_or(&toplevel);
    let mut entries: Vec<String> = fs::read_dir(parent)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|s| s.strip_prefix("bbb-wt-"))
                .map(String::from)
        })
        .collect();
    entries.sort();

    if entries.is_empty() {
        println!("no worker worktrees found");
    } else {
        for lane in entries {
            print_status_one(&toplevel, &lane)?;
            println!();
        }
    }

    if let Ok(file) = target_pool_lease_file(&toplevel) {
        let leases = read_leases(&file);
        if !leases.is_empty() {
            println!("target leases:");
            for (slot, lane) in leases {
                println!("  {} -> {lane}", target_pool_slot_dir(slot).display());
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn worker_cleanup(root: &Path, name: &str, flags: &[String]) -> Result<ExitCode, XtaskError> {
    validate_worker_name(name)?;
    let mut force = false;
    for flag in flags {
        match flag.as_str() {
            "--force" | "-f" => force = true,
            other => {
                return Err(XtaskError::usage(format!(
                    "unknown cleanup option: {other}"
                )));
            }
        }
    }

    let toplevel = worker_primary_root(root)?;
    let path = worktree_path(&toplevel, name)?;
    let mut branch = String::new();
    let mut branch_cleanup_failed = false;

    if !path.exists() {
        println!("worktree not found: {}", path.display());
    } else {
        if !is_git_worktree(&path) {
            return Err(XtaskError::usage(format!(
                "refusing to remove non-git worktree path: {}",
                path.display()
            )));
        }
        let current_branch = git_current_branch(&path);
        if !is_worker_branch_for_name(name, &current_branch) {
            let actual = if current_branch.is_empty() {
                "detached HEAD"
            } else {
                &current_branch
            };
            return Err(XtaskError::usage(format!(
                "refusing to remove worktree on unexpected branch: {}\n\
                 expected: worker/{name} or timestamp-suffixed variant\n\
                 actual: {actual}",
                path.display()
            )));
        }
        branch = current_branch;
        let dirty = Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(["status", "--short"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        if !dirty.is_empty() && !force {
            return Err(XtaskError::usage(format!(
                "refusing to remove dirty worktree: {}\n{dirty}\n\
                 rerun with --force only if these changes can be discarded",
                path.display()
            )));
        }

        let mut rm_cmd = Command::new("git");
        rm_cmd.arg("-C").arg(&toplevel).args(["worktree", "remove"]);
        if force {
            rm_cmd.arg("--force");
        }
        rm_cmd.arg(&path);
        let status = rm_cmd
            .status()
            .map_err(|err| XtaskError::failed(format!("git worktree remove: {err}")))?;
        if !status.success() {
            return Err(XtaskError::failed(format!(
                "git worktree remove exited with {status}"
            )));
        }
        println!("removed worktree: {}", path.display());
    }

    if !branch.is_empty() && branch_exists(&toplevel, &branch) {
        let delete_flag = if force { "-D" } else { "-d" };
        let status = Command::new("git")
            .arg("-C")
            .arg(&toplevel)
            .args(["branch", delete_flag, &branch])
            .status()
            .map_err(|err| XtaskError::failed(format!("git branch {delete_flag}: {err}")))?;
        if status.success() {
            println!("deleted branch: {branch}");
        } else {
            eprintln!("branch kept because git refused safe deletion: {branch}");
            eprintln!("review/integrate it before deleting manually");
            branch_cleanup_failed = true;
        }
    } else if !branch.is_empty() {
        println!("branch not found: {branch}");
    } else {
        println!("branch cleanup skipped because no worker branch was identified");
    }

    if let Some(target) = target_pool_lookup(&toplevel, name) {
        target_pool_release(&toplevel, name)?;
        println!("released target lease: {}", target.display());
    }
    if branch_cleanup_failed {
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

fn worker_release(root: &Path, name: &str) -> Result<ExitCode, XtaskError> {
    validate_worker_name(name)?;
    let toplevel = worker_primary_root(root)?;
    if let Some(target) = target_pool_lookup(&toplevel, name) {
        target_pool_release(&toplevel, name)?;
        println!("released target lease: {}", target.display());
    } else {
        println!("no target lease for lane: {name}");
    }
    Ok(ExitCode::SUCCESS)
}

fn worker_env_branch(toplevel: &Path, name: &str) -> Result<String, XtaskError> {
    let path = worktree_path(toplevel, name)?;
    if is_git_worktree(&path) {
        let cur = git_current_branch(&path);
        if !cur.is_empty() {
            return Ok(cur);
        }
    }
    Ok(format!("worker/{name}"))
}

fn resolve_existing_worker_env(
    root: &Path,
    name: &str,
) -> Result<(PathBuf, PathBuf, PathBuf, Option<PathBuf>), XtaskError> {
    validate_worker_name(name)?;
    let toplevel = worker_primary_root(root)?;
    let path = worktree_path(&toplevel, name)?;
    if !is_git_worktree(&path) {
        return Err(XtaskError::usage(format!(
            "worker worktree not found: {}",
            path.display()
        )));
    }
    let Some(target) = target_pool_lookup(&toplevel, name) else {
        return Err(XtaskError::usage(format!(
            "worker target lease not found: {name}"
        )));
    };
    let vanilla_root = resolve_worker_vanilla_root(&toplevel)?;
    Ok((toplevel, path, target, vanilla_root))
}

fn worker_env(root: &Path, name: &str, shell: ShellKind) -> Result<ExitCode, XtaskError> {
    let (toplevel, path, target, vanilla_root) = resolve_existing_worker_env(root, name)?;
    let branch = worker_env_branch(&toplevel, name)?;
    print_worker_summary(name, &path, &branch, &target, vanilla_root.as_deref());
    println!("\nExport for focused tests:");
    for line in format_shell_exports(shell, None, name, &target, vanilla_root.as_deref()) {
        println!("  {line}");
    }
    Ok(ExitCode::SUCCESS)
}

fn worker_shell_env(root: &Path, name: &str, shell: ShellKind) -> Result<ExitCode, XtaskError> {
    let (_toplevel, path, target, vanilla_root) = resolve_existing_worker_env(root, name)?;
    for line in format_shell_exports(shell, Some(&path), name, &target, vanilla_root.as_deref()) {
        println!("{line}");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_env_and_shell_env_are_read_only_for_missing_workers() {
        let tmp = env::temp_dir().join(format!(
            "bbb-xtask-worker-env-readonly-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        let repo = tmp.join("bbb");
        fs::create_dir_all(&repo).unwrap();
        let init = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["init", "-q"])
            .status()
            .unwrap();
        assert!(init.success());

        let lease_file = target_pool_lease_file(&repo).unwrap();
        assert!(!lease_file.exists());

        let env_err = worker_env(&repo, "unknown_worker", ShellKind::Sh).unwrap_err();
        assert!(env_err.message().contains("worker worktree not found"));
        assert!(!lease_file.exists());

        let shell_err = worker_shell_env(&repo, "unknown_worker", ShellKind::Sh).unwrap_err();
        assert!(shell_err.message().contains("worker worktree not found"));
        assert!(!lease_file.exists());

        // Pre-populate an unrelated lease and verify queries never mutate it.
        write_leases(&lease_file, &[(1, "existing_lane".to_string())]).unwrap();
        let before = fs::read_to_string(&lease_file).unwrap();
        assert!(worker_env(&repo, "unknown_worker", ShellKind::Sh).is_err());
        assert!(worker_shell_env(&repo, "unknown_worker", ShellKind::Sh).is_err());

        // Existing git worktree with no lease must also fail read-only without allocating a slot.
        let unleased_wt = tmp.join("bbb-wt-unleased");
        fs::create_dir_all(&unleased_wt).unwrap();
        assert!(Command::new("git")
            .arg("-C")
            .arg(&unleased_wt)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success());
        let unleased_err = worker_env(&repo, "unleased", ShellKind::Sh).unwrap_err();
        assert!(unleased_err
            .message()
            .contains("worker target lease not found"));
        assert!(worker_shell_env(&repo, "unleased", ShellKind::Sh).is_err());
        assert_eq!(fs::read_to_string(&lease_file).unwrap(), before);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn non_sibling_worktree_preserves_active_leases_and_resolves_primary_paths() {
        let tmp = env::temp_dir().join(format!(
            "bbb-xtask-worker-nonsibling-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        let primary = tmp.join("primary").join("bbb");
        let external_parent = tmp.join("external");
        let share_wt = external_parent.join("share-wt");
        let active_wt = tmp.join("primary").join("bbb-wt-active");
        fs::create_dir_all(&primary).unwrap();
        fs::create_dir_all(&external_parent).unwrap();

        let run_git = |cwd: &Path, args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(cwd)
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

        run_git(&primary, &["init", "-q"]);
        run_git(&primary, &["config", "user.name", "Worker Test"]);
        run_git(
            &primary,
            &["config", "user.email", "worker-test@bbb.invalid"],
        );
        fs::write(primary.join("README.md"), "seed\n").unwrap();
        run_git(&primary, &["add", "README.md"]);
        run_git(&primary, &["commit", "-qm", "seed"]);

        // Create a dummy primary .venv python so resolve_existing_worker_env succeeds without Python.
        #[cfg(windows)]
        {
            let py = primary.join(".venv").join("Scripts").join("python.exe");
            fs::create_dir_all(py.parent().unwrap()).unwrap();
            fs::write(&py, b"").unwrap();
        }
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            let py = primary.join(".venv").join("bin").join("python");
            fs::create_dir_all(py.parent().unwrap()).unwrap();
            fs::write(&py, "#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(&py, fs::Permissions::from_mode(0o755)).unwrap();
        }

        run_git(
            &primary,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "worker/active",
                active_wt.to_str().unwrap(),
                "HEAD",
            ],
        );
        run_git(
            &primary,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "share-branch",
                share_wt.to_str().unwrap(),
                "HEAD",
            ],
        );

        let lease_file = target_pool_lease_file(&primary).unwrap();
        write_leases(&lease_file, &[(1, "active".to_string())]).unwrap();

        let resolved_primary = worker_primary_root(&share_wt).unwrap();
        assert_eq!(
            fs::canonicalize(&resolved_primary).unwrap(),
            fs::canonicalize(&primary).unwrap()
        );

        // Leasing and releasing from the non-sibling worktree must keep active's slot intact.
        let second_slot = target_pool_lease(&share_wt, "second").unwrap();
        assert_eq!(second_slot, target_pool_slot_dir(2));
        assert_eq!(
            read_leases(&lease_file),
            vec![(1, "active".to_string()), (2, "second".to_string())]
        );

        target_pool_release(&share_wt, "second").unwrap();
        assert_eq!(read_leases(&lease_file), vec![(1, "active".to_string())]);

        let (_root, wt_path, target, _vanilla) =
            resolve_existing_worker_env(&share_wt, "active").unwrap();
        assert_eq!(
            fs::canonicalize(&wt_path).unwrap(),
            fs::canonicalize(&active_wt).unwrap()
        );
        assert_eq!(target, target_pool_slot_dir(1));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn worker_wasm_and_web_dirs_are_scoped_per_target_slot() {
        let slot = Path::new("/pool/bbb-target-lane-3");
        assert_eq!(
            worker_wasm_target_dir(slot),
            PathBuf::from("/pool/bbb-target-wasm-lane-3")
        );
        assert_eq!(
            worker_web_out_dir(slot),
            PathBuf::from("/pool/bbb-web-dist-lane-3")
        );
    }
}
