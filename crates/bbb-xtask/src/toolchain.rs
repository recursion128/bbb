//! Cross-platform toolchain, Git worktree, target-cache, and Python runner helpers.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, ExitCode, ExitStatus},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MIN_TEST_STACK_BYTES: &str = "16777216";

#[derive(Debug)]
pub enum XtaskError {
    Usage(String),
    Failed(u8, String),
}

impl XtaskError {
    pub fn usage(msg: impl Into<String>) -> Self {
        Self::Usage(msg.into())
    }

    pub fn failed(msg: impl Into<String>) -> Self {
        Self::Failed(1, msg.into())
    }

    pub fn with_code(code: u8, msg: impl Into<String>) -> Self {
        Self::Failed(code, msg.into())
    }

    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Usage(_) => ExitCode::from(2),
            Self::Failed(code, _) => ExitCode::from(*code),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Usage(msg) | Self::Failed(_, msg) => msg,
        }
    }
}

pub fn normalize_git_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if cfg!(windows) {
        PathBuf::from(trimmed.replace('/', "\\"))
    } else {
        PathBuf::from(trimmed)
    }
}

pub fn repo_root() -> PathBuf {
    if let Some(explicit) = env::var_os("BBB_REPO_ROOT").filter(|v| !v.is_empty()) {
        return PathBuf::from(explicit);
    }
    if let Ok(cwd) = env::current_dir() {
        if let Ok(out) = Command::new("git")
            .arg("-C")
            .arg(&cwd)
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if !text.trim().is_empty() {
                    return normalize_git_path(&text);
                }
            }
        }
        if cwd.join("Cargo.toml").is_file() || cwd.join("scripts").is_dir() {
            return cwd;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/bbb-xtask sits two levels under workspace root")
        .to_path_buf()
}

pub fn git_common_dir(root: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    if text.trim().is_empty() {
        return None;
    }
    Some(normalize_git_path(&text))
}

pub fn primary_worktree_root(root: &Path) -> Result<PathBuf, XtaskError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|err| XtaskError::usage(format!("run git worktree list: {err}")))?;
    if out.status.success() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("worktree ") {
                let primary = normalize_git_path(rest);
                if primary.is_dir() {
                    return Ok(primary);
                }
            }
        }
    }
    Err(XtaskError::usage(format!(
        "cannot resolve the primary Git worktree from: {}",
        root.display()
    )))
}

pub fn exe_suffix() -> &'static str {
    if cfg!(windows) {
        ".exe"
    } else {
        ""
    }
}

pub fn find_in_path(bin_name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    let with_ext = format!("{bin_name}{}", exe_suffix());
    for dir in env::split_paths(&path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(&with_ext);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        if cfg!(windows) {
            for ext in [".cmd", ".bat"] {
                let script = dir.join(format!("{bin_name}{ext}"));
                if script.is_file() {
                    return Some(script);
                }
            }
            let bare = dir.join(bin_name);
            if bare.is_file() {
                return Some(bare);
            }
        }
    }
    None
}

#[cfg(unix)]
pub fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
        .unwrap_or(false)
}

#[cfg(not(unix))]
pub fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

pub fn resolve_cargo_bin() -> Result<PathBuf, XtaskError> {
    if let Some(explicit) = env::var_os("CARGO").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(explicit);
        if is_executable_file(&path) {
            return Ok(path);
        }
    }
    if let Some(found) = find_in_path("cargo") {
        return Ok(found);
    }
    let cargo_home = env::var_os("CARGO_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .or_else(|| env::var_os("USERPROFILE"))
                .filter(|v| !v.is_empty())
                .map(|home| PathBuf::from(home).join(".cargo"))
        })
        .unwrap_or_else(|| PathBuf::from(".cargo"));
    let expected = cargo_home
        .join("bin")
        .join(format!("cargo{}", exe_suffix()));
    if is_executable_file(&expected) {
        return Ok(expected);
    }
    if cfg!(windows) {
        let cmd_shim = cargo_home.join("bin").join("cargo.cmd");
        if cmd_shim.is_file() {
            return Ok(cmd_shim);
        }
    }
    Err(XtaskError::usage(format!(
        "cargo not found: not on PATH and not at {}\n\
         install the pinned Rust toolchain with rustup, then rerun",
        expected.display()
    )))
}

pub fn path_with_toolchain_bin(cargo_bin: &Path) -> OsString {
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(bin_dir) = cargo_bin.parent() {
        paths.push(bin_dir.to_path_buf());
    }
    if let Some(existing) = env::var_os("PATH") {
        for entry in env::split_paths(&existing) {
            if !entry.as_os_str().is_empty() && !paths.iter().any(|p| p == &entry) {
                paths.push(entry);
            }
        }
    }
    env::join_paths(paths).unwrap_or_default()
}

pub fn validate_target_suffix(suffix: &str) -> Result<(), XtaskError> {
    if suffix.is_empty()
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(XtaskError::usage(format!(
            "target suffix may contain only ASCII letters, digits, '_' and '-': {suffix}"
        )));
    }
    Ok(())
}

pub fn target_root_override() -> Option<PathBuf> {
    env::var_os("BBB_TARGET_ROOT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub fn default_target_root() -> PathBuf {
    target_root_override().unwrap_or_else(|| {
        if cfg!(unix) {
            PathBuf::from("/tmp")
        } else {
            env::temp_dir()
        }
    })
}

pub fn default_target_dir_for_name(name: &str) -> PathBuf {
    default_target_root().join(format!("bbb-target-{name}"))
}

pub fn target_dir_for_arg_with_root(arg: &str, root: &Path) -> Result<PathBuf, XtaskError> {
    if arg.is_empty() {
        return Err(XtaskError::usage("target suffix must not be empty"));
    }
    let path = Path::new(arg);
    if path.parent() == Some(root) {
        if let Some(suffix) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix("bbb-target-"))
        {
            validate_target_suffix(suffix)?;
            return Ok(root.join(format!("bbb-target-{suffix}")));
        }
    }
    if let Some(suffix) = arg.strip_prefix("/tmp/bbb-target-") {
        validate_target_suffix(suffix)?;
        return Ok(PathBuf::from(format!("/tmp/bbb-target-{suffix}")));
    }
    if arg.contains('/') || arg.contains('\\') {
        return Err(XtaskError::usage(format!(
            "target suffix must be a name or {} path: {arg}",
            root.join("bbb-target-<name>").display()
        )));
    }
    validate_target_suffix(arg)?;
    Ok(root.join(format!("bbb-target-{arg}")))
}

pub fn target_dir_for_arg(arg: &str) -> Result<PathBuf, XtaskError> {
    target_dir_for_arg_with_root(arg, &default_target_root())
}

pub fn resolve_target_dir() -> Result<PathBuf, XtaskError> {
    if let Some(explicit) = env::var_os("CARGO_TARGET_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(explicit));
    }
    let name = env::var("BBB_CARGO_TARGET_NAME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "main".to_string());
    validate_target_suffix(&name)?;
    Ok(default_target_dir_for_name(&name))
}

pub fn resolve_vanilla_root_with<F>(explicit: Option<&Path>, platform: &str, get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString>,
{
    if let Some(dir) = explicit {
        return dir.to_path_buf();
    }
    if let Some(val) = get_env("BBB_VANILLA_ROOT").filter(|v| !v.is_empty()) {
        return PathBuf::from(val);
    }
    if platform == "windows" {
        if let Some(local) = get_env("LOCALAPPDATA").filter(|v| !v.is_empty()) {
            return PathBuf::from(local).join("bbb").join("vanilla");
        }
        if let Some(home) = get_env("USERPROFILE")
            .or_else(|| get_env("HOME"))
            .filter(|v| !v.is_empty())
        {
            return PathBuf::from(home)
                .join("AppData")
                .join("Local")
                .join("bbb")
                .join("vanilla");
        }
    } else if platform == "macos" {
        if let Some(home) = get_env("HOME").filter(|v| !v.is_empty()) {
            return PathBuf::from(home)
                .join("Library")
                .join("Caches")
                .join("bbb")
                .join("vanilla");
        }
    } else {
        if let Some(xdg) = get_env("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
            return PathBuf::from(xdg).join("bbb").join("vanilla");
        }
        if let Some(home) = get_env("HOME").filter(|v| !v.is_empty()) {
            return PathBuf::from(home)
                .join(".cache")
                .join("bbb")
                .join("vanilla");
        }
    }
    env::temp_dir().join("bbb-vanilla")
}

pub fn resolve_vanilla_root(explicit: Option<&Path>) -> PathBuf {
    resolve_vanilla_root_with(explicit, env::consts::OS, |key| env::var_os(key))
}

pub fn is_prepared_vanilla_dir(candidate: &Path) -> bool {
    candidate.join("state/26.1.json").is_file()
        || candidate.join("sources/26.1/state.json").is_file()
        || candidate.join("sources/26.1/version.json").is_file()
        || candidate.join("sources/26.1/assets/minecraft").is_dir()
}

pub fn maybe_prepared_vanilla_root() -> Option<PathBuf> {
    if let Some(val) = env::var_os("BBB_VANILLA_ROOT").filter(|v| !v.is_empty()) {
        let explicit = PathBuf::from(val);
        return is_prepared_vanilla_dir(&explicit).then_some(explicit);
    }
    let candidate = resolve_vanilla_root(None);
    is_prepared_vanilla_dir(&candidate).then_some(candidate)
}

pub fn cargo_command(root: &Path, cargo_bin: &Path, target_dir: &Path) -> Command {
    let mut cmd = Command::new(cargo_bin);
    cmd.current_dir(root);
    cmd.env("CARGO", cargo_bin);
    cmd.env("PATH", path_with_toolchain_bin(cargo_bin));
    cmd.env("CARGO_TARGET_DIR", target_dir);
    cmd
}

pub fn status_to_exit_code(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(0) => ExitCode::SUCCESS,
        Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        None => ExitCode::from(1),
    }
}

pub fn parse_rustc_host_triple(rustc_vv: &str) -> Option<String> {
    rustc_vv
        .lines()
        .find_map(|line| {
            line.strip_prefix("host:")
                .map(|rest| rest.trim().to_string())
        })
        .filter(|s| !s.is_empty())
}

pub fn host_triple_from_rustc(cargo_bin: &Path) -> Result<String, XtaskError> {
    let rustc_bin = cargo_bin
        .parent()
        .map(|dir| dir.join(format!("rustc{}", exe_suffix())))
        .filter(|p| is_executable_file(p))
        .unwrap_or_else(|| PathBuf::from("rustc"));
    let output = Command::new(&rustc_bin)
        .env("PATH", path_with_toolchain_bin(cargo_bin))
        .arg("-vV")
        .output()
        .map_err(|err| XtaskError::failed(format!("run rustc -vV: {err}")))?;
    if !output.status.success() {
        return Err(XtaskError::failed(format!(
            "rustc -vV exited with {}",
            output.status
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_rustc_host_triple(&text)
        .ok_or_else(|| XtaskError::failed("missing `host:` line in `rustc -vV`"))
}

pub fn venv_python_path(project_root: &Path) -> Option<PathBuf> {
    if cfg!(windows) {
        let win_py = project_root
            .join(".venv")
            .join("Scripts")
            .join("python.exe");
        if win_py.is_file() {
            return Some(win_py);
        }
    }
    let unix_py = project_root.join(".venv").join("bin").join("python");
    if is_executable_file(&unix_py) {
        return Some(unix_py);
    }
    let win_py = project_root
        .join(".venv")
        .join("Scripts")
        .join("python.exe");
    if win_py.is_file() {
        return Some(win_py);
    }
    None
}

fn find_system_python() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["python", "python3"]
    } else {
        &["python3", "python"]
    };
    let path_var = env::var_os("PATH")?;
    for name in names {
        let with_ext = format!("{name}{}", exe_suffix());
        for dir in env::split_paths(&path_var) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            if cfg!(windows)
                && dir
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains("windowsapps")
            {
                continue;
            }
            let candidate = dir.join(&with_ext);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn prepare_python_command(
    worktree_root: &Path,
    args: &[String],
) -> Result<Command, XtaskError> {
    if args.is_empty() {
        return Err(XtaskError::usage(
            "usage: cargo dev python <python-command-or-script> [args...]",
        ));
    }
    let primary =
        primary_worktree_root(worktree_root).unwrap_or_else(|_| worktree_root.to_path_buf());
    let pythonpath = worktree_root.join("scripts");
    let configure_common = |cmd: &mut Command| {
        cmd.current_dir(worktree_root)
            .env("PYTHONPATH", &pythonpath);
        if env::var_os("BBB_XTASK_BIN").is_none() {
            if let Ok(exe) = env::current_exe() {
                cmd.env("BBB_XTASK_BIN", exe);
            }
        }
    };

    let is_direct_python_arg = args[0].starts_with('-')
        || args[0].ends_with(".py")
        || Path::new(&args[0]).components().count() > 1;

    if let Some(uv_bin) = find_in_path("uv") {
        if venv_python_path(&primary).is_some()
            && primary.join("pyproject.toml").is_file()
            && primary.join("uv.lock").is_file()
        {
            let mut cmd = Command::new(uv_bin);
            configure_common(&mut cmd);
            cmd.args([
                "run",
                "--offline",
                "--locked",
                "--exact",
                "--no-python-downloads",
                "--project",
            ])
            .arg(&primary)
            .arg("--");
            if is_direct_python_arg {
                cmd.arg("python");
            }
            cmd.args(args);
            return Ok(cmd);
        }
    }

    if let Some(venv_py) = venv_python_path(&primary) {
        let mut cmd = Command::new(venv_py);
        configure_common(&mut cmd);
        if args[0] == "python" {
            cmd.args(&args[1..]);
        } else if is_direct_python_arg {
            cmd.args(args);
        } else {
            cmd.arg("-m").args(args);
        }
        return Ok(cmd);
    }

    let sys_py = find_system_python();
    if args[0] == "python" {
        if let Some(py) = sys_py {
            let mut cmd = Command::new(py);
            configure_common(&mut cmd);
            cmd.args(&args[1..]);
            return Ok(cmd);
        }
    } else if is_direct_python_arg {
        if let Some(py) = sys_py {
            let mut cmd = Command::new(py);
            configure_common(&mut cmd);
            cmd.args(args);
            return Ok(cmd);
        }
    } else if let Some(tool) = find_in_path(&args[0]) {
        let mut cmd = Command::new(tool);
        configure_common(&mut cmd);
        cmd.args(&args[1..]);
        return Ok(cmd);
    }

    Err(XtaskError::usage(format!(
        "bbb Python environment is not prepared in the primary worktree\n\
         run: cargo dev bootstrap (from {})",
        primary.display()
    )))
}

pub fn run_python_tool(worktree_root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut cmd = prepare_python_command(worktree_root, args)?;
    let status = cmd
        .status()
        .map_err(|err| XtaskError::failed(format!("invoke python tool: {err}")))?;
    Ok(status_to_exit_code(status))
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m as u32, d as u32)
}

fn utc_parts_now() -> (i64, u32, u32, u32, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400) as u32;
    let (year, month, day) = civil_from_days(days);
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    (year, month, day, hour, minute, second)
}

pub fn utc_iso8601_now() -> String {
    let (y, m, d, hh, mm, ss) = utc_parts_now();
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

pub fn utc_compact_timestamp() -> String {
    let (y, m, d, hh, mm, ss) = utc_parts_now();
    format!("{y:04}{m:02}{d:02}{hh:02}{mm:02}{ss:02}")
}
