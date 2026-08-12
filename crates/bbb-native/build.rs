use std::{env, path::PathBuf, process::Command};

fn git(repo: &PathBuf, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git must be available while building bbb-native");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output must be UTF-8")
        .trim()
        .to_owned()
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let repo = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let commit = git(&repo, &["rev-parse", "HEAD"]);
    let status = git(&repo, &["status", "--porcelain", "--untracked-files=no"]);
    let git_dir = PathBuf::from(git(&repo, &["rev-parse", "--absolute-git-dir"]));
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    if let Ok(reference) = std::fs::read_to_string(git_dir.join("HEAD")) {
        if let Some(reference) = reference.strip_prefix("ref: ").map(str::trim) {
            println!(
                "cargo:rerun-if-changed={}",
                git_dir.join(reference).display()
            );
        }
    }
    for tracked in git(&repo, &["ls-files"]).lines() {
        println!("cargo:rerun-if-changed={}", repo.join(tracked).display());
    }
    println!("cargo:rustc-env=BBB_SOURCE_COMMIT={commit}");
    println!(
        "cargo:rustc-env=BBB_SOURCE_DIRTY={}",
        if status.is_empty() { "false" } else { "true" }
    );
}
