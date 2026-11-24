use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

fn git(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().to_owned())
}

fn join_normalized(base: &Path, rel: &Path) -> PathBuf {
    let mut out = if rel.is_absolute() {
        PathBuf::new()
    } else {
        base.to_path_buf()
    };
    for component in rel.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn resolve_git_dir(repo: &Path) -> Option<PathBuf> {
    let dot_git = repo.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    if !dot_git.is_file() {
        return None;
    }
    let content = fs::read_to_string(&dot_git).ok()?;
    let raw = content
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:").map(str::trim))
        .filter(|path| !path.is_empty())?;
    Some(join_normalized(repo, Path::new(raw)))
}

fn resolve_common_git_dir(git_dir: &Path) -> PathBuf {
    if let Ok(content) = fs::read_to_string(git_dir.join("commondir")) {
        let raw = content.trim();
        if !raw.is_empty() {
            return join_normalized(git_dir, Path::new(raw));
        }
    }
    git_dir.to_path_buf()
}

fn emit_git_rerun_directives(git_dir: &Path) {
    let head = git_dir.join("HEAD");
    if head.is_file() {
        println!("cargo:rerun-if-changed={}", head.display());
    }
    let common_dir = resolve_common_git_dir(git_dir);
    if let Ok(head_contents) = fs::read_to_string(&head) {
        if let Some(reference) = head_contents.strip_prefix("ref:").map(str::trim) {
            if !reference.is_empty() {
                let common_ref = join_normalized(&common_dir, Path::new(reference));
                if common_ref.is_file() {
                    println!("cargo:rerun-if-changed={}", common_ref.display());
                } else if common_dir != git_dir {
                    let local_ref = join_normalized(git_dir, Path::new(reference));
                    if local_ref.is_file() {
                        println!("cargo:rerun-if-changed={}", local_ref.display());
                    }
                }
            }
        }
    }
    let packed_refs = common_dir.join("packed-refs");
    if packed_refs.is_file() {
        println!("cargo:rerun-if-changed={}", packed_refs.display());
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest = manifest_dir.join("windows.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-bin=bbb-client=/STACK:16777216");
        println!("cargo:rustc-link-arg-bin=bbb-client=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=bbb-client=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }

    let (commit, dirty) = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(|repo| {
            let git_dir = resolve_git_dir(repo)?;
            emit_git_rerun_directives(&git_dir);
            let commit = git(repo, &["rev-parse", "HEAD"])
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".to_owned());
            let dirty = git(repo, &["status", "--porcelain", "--untracked-files=no"])
                .is_some_and(|status| !status.is_empty());
            Some((commit, dirty))
        })
        .unwrap_or_else(|| ("unknown".to_owned(), false));

    println!("cargo:rustc-env=BBB_SOURCE_COMMIT={commit}");
    println!(
        "cargo:rustc-env=BBB_SOURCE_DIRTY={}",
        if dirty { "true" } else { "false" }
    );
}
