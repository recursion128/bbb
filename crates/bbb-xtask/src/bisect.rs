//! First-parent commit bisection runner (`cargo dev batch-gate`).

use std::{
    collections::HashMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use crate::{
    gate::shell_command,
    toolchain::{
        default_target_root, git_common_dir, normalize_git_path, target_dir_for_arg,
        utc_iso8601_now, XtaskError,
    },
};

struct TempBatchWorktree {
    repo_root: PathBuf,
    batch_root: PathBuf,
    worktree_dir: PathBuf,
}

impl Drop for TempBatchWorktree {
    fn drop(&mut self) {
        if self.worktree_dir.exists() {
            let _ = Command::new("git")
                .arg("-C")
                .arg(&self.repo_root)
                .args(["worktree", "remove", "--force"])
                .arg(&self.worktree_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = fs::remove_dir_all(&self.batch_root);
    }
}

pub(crate) fn git_stdout(root: &Path, args: &[&str]) -> Result<String, XtaskError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|err| XtaskError::usage(format!("run git {}: {err}", args.join(" "))))?;
    if !out.status.success() {
        return Err(XtaskError::usage(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn print_batch_gate_usage() {
    eprintln!(
        "\
Usage:
  cargo dev batch-gate [<good-commit>..<bad-commit>]

Bisects a first-parent commit range using `cargo dev gate` in an isolated
temporary worktree to identify the first failing commit. When omitted, defaults
to `<last-gated>..HEAD` from `.git/bbb/last-gated`."
    );
}

pub fn run_batch_gate(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_batch_gate_usage();
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() > 1 {
        return Err(XtaskError::usage(
            "batch-gate accepts at most one <good-commit>..<bad-commit> range",
        ));
    }

    let git_toplevel = git_stdout(root, &["rev-parse", "--show-toplevel"])
        .map(|s| normalize_git_path(&s))
        .map_err(|_| XtaskError::usage("batch-gate must run inside a Git worktree"))?;

    let range = if let Some(explicit) = args.first() {
        explicit.clone()
    } else {
        let common = git_common_dir(&git_toplevel)
            .ok_or_else(|| XtaskError::usage("batch-gate must run inside a Git worktree"))?;
        let last_gated_file = common.join("bbb").join("last-gated");
        if !last_gated_file.is_file() {
            return Err(XtaskError::usage(format!(
                "batch-gate found no recorded last-gated commit ({}); \
                 pass an explicit <good-commit>..<bad-commit> range",
                last_gated_file.display()
            )));
        }
        let stamp = fs::read_to_string(&last_gated_file).map_err(|err| {
            XtaskError::failed(format!("read {}: {err}", last_gated_file.display()))
        })?;
        let default_range = format!("{}..HEAD", stamp.trim());
        println!("batch-gate: defaulting to last-gated range: {default_range}");
        default_range
    };

    if range.contains("...") {
        return Err(XtaskError::usage(format!(
            "batch-gate requires a two-dot range, not a symmetric three-dot range: {range}"
        )));
    }
    let Some((good_ref, bad_ref)) = range.split_once("..") else {
        return Err(XtaskError::usage(format!(
            "batch-gate range must have the form <good-commit>..<bad-commit>: {range}"
        )));
    };
    if good_ref.is_empty() || bad_ref.is_empty() {
        return Err(XtaskError::usage(format!(
            "batch-gate range endpoints must not be empty: {range}"
        )));
    }

    let good_commit = git_stdout(
        &git_toplevel,
        &["rev-parse", "--verify", &format!("{good_ref}^{{commit}}")],
    )
    .map_err(|_| {
        XtaskError::usage(format!(
            "batch-gate cannot resolve good endpoint: {good_ref}"
        ))
    })?;
    let bad_commit = git_stdout(
        &git_toplevel,
        &["rev-parse", "--verify", &format!("{bad_ref}^{{commit}}")],
    )
    .map_err(|_| XtaskError::usage(format!("batch-gate cannot resolve bad endpoint: {bad_ref}")))?;
    if good_commit == bad_commit {
        return Err(XtaskError::usage(format!(
            "batch-gate endpoints resolve to the same commit: {good_commit}"
        )));
    }

    let ancestor_ok = Command::new("git")
        .arg("-C")
        .arg(&git_toplevel)
        .args(["merge-base", "--is-ancestor", &good_commit, &bad_commit])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ancestor_ok {
        return Err(XtaskError::usage(format!(
            "batch-gate good endpoint is not an ancestor of bad endpoint\ngood: {good_commit}\nbad:  {bad_commit}"
        )));
    }

    let first_parents = git_stdout(&git_toplevel, &["rev-list", "--first-parent", &bad_commit])?;
    if !first_parents.lines().any(|line| line.trim() == good_commit) {
        return Err(XtaskError::usage(
            "batch-gate good endpoint is not on the bad endpoint's first-parent chain\n\
             use a linear/first-parent range so 'first bad commit' is unambiguous",
        ));
    }

    let custom_cmd = env::var("BBB_BATCH_GATE_COMMAND")
        .ok()
        .filter(|s| !s.is_empty());
    if custom_cmd.is_some() && env::var("BBB_BATCH_GATE_TEST_MODE").as_deref() != Ok("1") {
        return Err(XtaskError::usage(
            "BBB_BATCH_GATE_COMMAND is reserved for isolated script tests",
        ));
    }
    let authoritative_label = custom_cmd
        .clone()
        .unwrap_or_else(|| "cargo dev gate".to_string());

    let batch_target_name = env::var("BBB_BATCH_GATE_TARGET_NAME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "batch-gate".to_string());
    let batch_target_dir = target_dir_for_arg(&batch_target_name)?;

    let pid = std::process::id();
    let stamp = utc_iso8601_now().replace([':', '-'], "");
    let batch_root = default_target_root().join(format!("bbb-batch-gate-{stamp}-{pid}"));
    let batch_logs = match env::var_os("BBB_BATCH_GATE_LOG_DIR").filter(|v| !v.is_empty()) {
        Some(dir) => {
            let p = PathBuf::from(dir);
            if !p.is_absolute() {
                return Err(XtaskError::usage(
                    "BBB_BATCH_GATE_LOG_DIR must be an absolute path",
                ));
            }
            p
        }
        None => env::temp_dir().join(format!("bbb-batch-gate-logs-{stamp}-{pid}")),
    };
    fs::create_dir_all(&batch_root)
        .map_err(|err| XtaskError::failed(format!("create {}: {err}", batch_root.display())))?;
    fs::create_dir_all(&batch_logs)
        .map_err(|err| XtaskError::failed(format!("create {}: {err}", batch_logs.display())))?;

    let guard = TempBatchWorktree {
        repo_root: git_toplevel.clone(),
        worktree_dir: batch_root.join("worktree"),
        batch_root: batch_root.clone(),
    };

    let rev_list = git_stdout(
        &git_toplevel,
        &[
            "rev-list",
            "--first-parent",
            "--reverse",
            &format!("{good_commit}..{bad_commit}"),
        ],
    )?;
    let mut commits = vec![good_commit.clone()];
    commits.extend(rev_list.lines().filter(|s| !s.is_empty()).map(String::from));
    let last_index = commits.len() - 1;

    println!("batch-gate range: {good_commit}..{bad_commit}");
    println!("batch-gate first-parent commits: {last_index}");
    println!(
        "batch-gate temporary worktree: {}",
        guard.worktree_dir.display()
    );
    println!("batch-gate logs: {}", batch_logs.display());
    println!("batch-gate target: {}", batch_target_dir.display());
    println!("batch-gate authoritative command: {authoritative_label}");

    let add_status = Command::new("git")
        .arg("-C")
        .arg(&git_toplevel)
        .args(["worktree", "add", "--detach", "--quiet"])
        .arg(&guard.worktree_dir)
        .arg(&bad_commit)
        .status()
        .map_err(|err| XtaskError::failed(format!("git worktree add: {err}")))?;
    if !add_status.success() {
        return Err(XtaskError::failed("git worktree add --detach failed"));
    }

    let self_exe = env::current_exe()
        .map_err(|err| XtaskError::failed(format!("resolve current_exe: {err}")))?;
    let mut cache: HashMap<String, i32> = HashMap::new();

    let mut test_commit = |commit: &str| -> Result<i32, XtaskError> {
        if let Some(&cached) = cache.get(commit) {
            return Ok(cached);
        }
        let short = git_stdout(&git_toplevel, &["rev-parse", "--short=12", commit])?;
        let log_file = batch_logs.join(format!("{short}.log"));
        let status = Command::new("git")
            .arg("-C")
            .arg(&guard.worktree_dir)
            .args(["checkout", "--detach", "--quiet", commit])
            .status()
            .map_err(|err| XtaskError::failed(format!("git checkout {commit}: {err}")))?;
        if !status.success() {
            return Err(XtaskError::failed(format!("git checkout {commit} failed")));
        }
        let subject = git_stdout(&guard.worktree_dir, &["log", "-1", "--format=%s"])?;

        let mut log = fs::File::create(&log_file)
            .map_err(|err| XtaskError::failed(format!("create {}: {err}", log_file.display())))?;
        writeln!(
            log,
            "commit: {commit}\nsubject: {subject}\ncommand: {authoritative_label}\n"
        )
        .ok();
        let log_err = log
            .try_clone()
            .map_err(|err| XtaskError::failed(format!("clone log handle: {err}")))?;

        let mut cmd = if let Some(ref custom) = custom_cmd {
            let mut c = shell_command(custom);
            c.env("BBB_BATCH_GATE_COMMIT", commit)
                .env("BBB_BATCH_GATE_WORKTREE", &guard.worktree_dir);
            c
        } else {
            let mut c = Command::new(&self_exe);
            c.arg("gate");
            c
        };
        cmd.current_dir(&guard.worktree_dir)
            .env_remove("BBB_REPO_ROOT")
            .env("BBB_CARGO_TARGET_NAME", &batch_target_name)
            .env("CARGO_TARGET_DIR", &batch_target_dir)
            .env("BBB_GATE_LOCALIZATION_ONLY", "1")
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err));

        let code = cmd
            .status()
            .map_err(|err| XtaskError::failed(format!("invoke batch-gate command: {err}")))?
            .code()
            .unwrap_or(1);
        cache.insert(commit.to_string(), code);
        if code == 0 {
            println!(
                "batch-gate: PASS {short} {subject} (log: {})",
                log_file.display()
            );
        } else {
            println!(
                "batch-gate: FAIL {short} {subject} (exit {code}; log: {})",
                log_file.display()
            );
        }
        Ok(code)
    };

    if test_commit(&commits[last_index])? == 0 {
        println!("batch-gate: range tip passed; no bisection needed");
        return Ok(ExitCode::SUCCESS);
    }
    if test_commit(&commits[0])? != 0 {
        return Err(XtaskError::failed(
            "batch-gate: baseline commit also fails; cannot attribute a regression within the range",
        ));
    }

    let mut low = 0usize;
    let mut high = last_index;
    while high - low > 1 {
        let mid = (low + high) / 2;
        if test_commit(&commits[mid])? == 0 {
            low = mid;
        } else {
            high = mid;
        }
    }

    let first_bad = &commits[high];
    let first_bad_subject = git_stdout(&git_toplevel, &["log", "-1", "--format=%s", first_bad])?;
    Err(XtaskError::failed(format!(
        "batch-gate: first bad commit: {first_bad} {first_bad_subject}"
    )))
}
