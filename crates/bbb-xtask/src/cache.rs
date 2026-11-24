//! Target-directory size reporting, cleanup, and warm-cache pruning (`cargo dev prune|size|clean-target`).

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use crate::{
    toolchain::{default_target_root, normalize_git_path, target_dir_for_arg, XtaskError},
    worker::is_pool_slot,
};

pub const PRUNE_KEEP_NAMES: &[&str] = &["main", "playable", "soak", "wasm", "wasm-bindgen-cli"];

fn dir_size_bytes(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries.flatten().map(|e| dir_size_bytes(&e.path())).sum()
}

fn format_human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut value = bytes as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx + 1 < UNITS.len() {
        value /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{bytes}B")
    } else {
        format!("{value:.1}{}", UNITS[idx])
    }
}

pub fn is_prune_keep(name: &str) -> bool {
    is_pool_slot(name)
        || name.strip_prefix("wasm-").is_some_and(is_pool_slot)
        || PRUNE_KEEP_NAMES.contains(&name)
}

pub fn print_size_usage() {
    eprintln!(
        "\
Usage:
  cargo dev size

Prints disk usage for every `bbb-target-*` cache directory under the active target root."
    );
}

pub fn print_clean_target_usage() {
    eprintln!(
        "\
Usage:
  cargo dev clean-target <name|bbb-target-path>

Deletes a single `bbb-target-<name>` build cache directory under the active target root."
    );
}

pub fn print_prune_usage() {
    eprintln!(
        "\
Usage:
  cargo dev prune [--force|-f]

Lists disposable `bbb-target-*` directories and any repo-local `target/` directory.
Without `--force`, runs in dry-run mode without deleting anything."
    );
}

pub fn run_size(args: &[String]) -> Result<ExitCode, XtaskError> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_size_usage();
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(other) = args.first() {
        return Err(XtaskError::usage(format!("unknown size option: {other}")));
    }
    let root = env::var_os("BBB_PRUNE_TARGET_ROOT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_target_root);
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("bbb-target-"))
        })
        .collect();
    dirs.sort();
    if dirs.is_empty() {
        println!("no bbb-target-* directories found under {}", root.display());
    } else {
        for dir in dirs {
            let size = format_human_size(dir_size_bytes(&dir));
            println!("{size:<8}\t{}", dir.display());
        }
    }
    Ok(ExitCode::SUCCESS)
}

pub fn run_clean_target(args: &[String]) -> Result<ExitCode, XtaskError> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_clean_target_usage();
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() != 1 {
        return Err(XtaskError::usage(
            "clean-target requires exactly one target suffix or bbb-target-<name> path",
        ));
    }
    let clean_target = target_dir_for_arg(&args[0])?;
    if clean_target.is_dir() {
        fs::remove_dir_all(&clean_target).map_err(|err| {
            XtaskError::failed(format!("remove {}: {err}", clean_target.display()))
        })?;
        println!("removed {}", clean_target.display());
    } else {
        println!("not found: {}", clean_target.display());
    }
    Ok(ExitCode::SUCCESS)
}

pub fn run_prune(root: &Path, args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut force = false;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                print_prune_usage();
                return Ok(ExitCode::SUCCESS);
            }
            "--force" | "-f" => force = true,
            other => {
                return Err(XtaskError::usage(format!("unknown prune option: {other}")));
            }
        }
    }

    let prune_root = match env::var_os("BBB_PRUNE_TARGET_ROOT").filter(|v| !v.is_empty()) {
        Some(val) => {
            let p = PathBuf::from(&val);
            if !p.is_absolute() {
                return Err(XtaskError::usage(format!(
                    "BBB_PRUNE_TARGET_ROOT must be an absolute path: {}",
                    p.display()
                )));
            }
            p
        }
        None => default_target_root(),
    };
    if !prune_root.is_dir() {
        return Err(XtaskError::usage(format!(
            "prune target root not found: {}",
            prune_root.display()
        )));
    }

    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|err| XtaskError::usage(format!("run git rev-parse: {err}")))?;
    if !out.status.success() {
        return Err(XtaskError::usage("prune must run inside a Git worktree"));
    }
    let git_toplevel = normalize_git_path(&String::from_utf8_lossy(&out.stdout));

    let mut candidates: Vec<PathBuf> = fs::read_dir(&prune_root)
        .map_err(|err| XtaskError::failed(format!("read {}: {err}", prune_root.display())))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("bbb-target-"))
        })
        .collect();
    candidates.sort();

    let mut delete_list: Vec<PathBuf> = Vec::new();
    if candidates.is_empty() {
        println!(
            "no bbb-target-* directories found under {}",
            prune_root.display()
        );
    } else {
        for dir in candidates {
            let name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("bbb-target-"))
                .unwrap_or("");
            let size = format_human_size(dir_size_bytes(&dir));
            if is_prune_keep(name) {
                println!("keep   {size:<8} {}", dir.display());
            } else {
                println!("delete {size:<8} {}", dir.display());
                delete_list.push(dir);
            }
        }
    }

    let repo_target = git_toplevel.join("target");
    if repo_target.is_dir() {
        let size = format_human_size(dir_size_bytes(&repo_target));
        println!(
            "delete {size:<8} {} (repo-local target; ignored by git)",
            repo_target.display()
        );
        delete_list.push(repo_target.clone());
    }

    if delete_list.is_empty() {
        println!("prune: nothing to delete");
        return Ok(ExitCode::SUCCESS);
    }

    if !force {
        println!();
        println!(
            "prune: dry-run; {} directories would be deleted",
            delete_list.len()
        );
        println!("rerun with --force to delete them");
        return Ok(ExitCode::SUCCESS);
    }

    for dir in &delete_list {
        if dir != &repo_target {
            let ok = dir.parent() == Some(prune_root.as_path())
                && dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("bbb-target-"));
            if !ok {
                return Err(XtaskError::usage(format!(
                    "refusing to remove path outside {}/bbb-target-*: {}",
                    prune_root.display(),
                    dir.display()
                )));
            }
        }
        fs::remove_dir_all(dir)
            .map_err(|err| XtaskError::failed(format!("remove {}: {err}", dir.display())))?;
        println!("removed {}", dir.display());
    }
    println!("prune: removed {} directories", delete_list.len());
    Ok(ExitCode::SUCCESS)
}
