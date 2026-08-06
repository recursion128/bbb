#!/usr/bin/env python3
"""Type-check the workspace for a second platform from the Linux dev host.

bbb declares a Linux release row and supports macOS development, but this
repository's gate runs on Linux only. Anything behind `cfg(target_os = "macos")`
is therefore code the gate never compiles, and it rots silently.

`cargo check --target aarch64-apple-darwin` closes most of that gap without an
Apple host. It cannot close all of it: some third-party build scripts compile C
or Objective-C for the target and need an Apple toolchain no Linux host has, so
every crate whose dependency closure reaches one of them stops at that build
script before any bbb source is read.

Which crates those are is measured, not modelled: this runs the check for every
workspace member and classifies each failure. A failure whose cargo error names
a known Apple-toolchain build script is reported as a blocked crate; anything
else is a real type error and fails the command. That way a crate that gains or
loses such a dependency re-partitions itself, and a genuine break can never be
mistaken for a toolchain limit.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys


DEFAULT_TARGET = "aarch64-apple-darwin"

# Build scripts that need a C/Objective-C toolchain and SDK for the target.
# The value is the reason reported to the developer, not a guess: each was
# observed failing this way on the Linux dev host.
APPLE_TOOLCHAIN_BUILD_SCRIPTS = {
    "ring": "compiles C and assembly for the target",
    "objc_exception": "compiles Objective-C against the Apple SDK",
    "coreaudio-sys": "runs bindgen against macOS SDK headers",
}

# The crate that exists precisely so the macOS system calls are compiled here.
# If it ever stops cross-checking, the extraction has lost its whole point, so
# that is an error rather than one more blocked crate.
REQUIRED_CROSS_CHECKED_CRATE = "bbb-platform-macos"

BUILD_SCRIPT_FAILURE_RE = re.compile(
    r"failed to run custom build command for `([A-Za-z0-9_-]+) v[^`]*`"
)


def workspace_members(repo_root: Path) -> list[str]:
    """Workspace member package names, in manifest order."""
    manifest = (repo_root / "Cargo.toml").read_text(encoding="utf-8")
    members: list[str] = []
    table = ""
    inside = False
    for line in manifest.splitlines():
        stripped = line.strip()
        if stripped.startswith("[") and not inside:
            table = stripped
            continue
        if table == "[workspace]" and stripped.startswith("members"):
            inside = True
            continue
        if inside:
            if stripped.startswith("]"):
                break
            member = stripped.strip('",')
            if member:
                members.append(Path(member).name)
    return members


def target_is_installed(target: str) -> bool:
    result = subprocess.run(
        ["rustup", "target", "list", "--installed"],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if result.returncode != 0:
        return False
    return target in result.stdout.split()


def classify_failure(cargo_output: str) -> tuple[str, str] | None:
    """The Apple-toolchain build script this failure stopped at, if any."""
    for match in BUILD_SCRIPT_FAILURE_RE.finditer(cargo_output):
        package = match.group(1)
        reason = APPLE_TOOLCHAIN_BUILD_SCRIPTS.get(package)
        if reason is not None:
            return package, reason
    return None


def check_crate(repo_root: Path, crate: str, target: str) -> tuple[bool, str]:
    result = subprocess.run(
        ["cargo", "check", "--quiet", "-p", crate, "--target", target],
        cwd=repo_root,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return result.returncode == 0, result.stdout


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", default=DEFAULT_TARGET)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="emit the partition as JSON instead of a report",
    )
    args = parser.parse_args(argv)

    crates = workspace_members(args.repo_root)
    if not crates:
        print("cross-check: no workspace members found", file=sys.stderr)
        return 2

    if not target_is_installed(args.target):
        print(
            f"cross-check: skipped, rustup target {args.target} is not installed "
            f"(add it with: rustup target add {args.target})"
        )
        return 0

    checked: list[str] = []
    blocked: list[tuple[str, str, str]] = []
    broken: list[tuple[str, str]] = []
    for crate in crates:
        ok, output = check_crate(args.repo_root, crate, args.target)
        if ok:
            checked.append(crate)
            continue
        blocker = classify_failure(output)
        if blocker is None:
            broken.append((crate, output))
        else:
            blocked.append((crate, blocker[0], blocker[1]))

    if args.json:
        print(
            json.dumps(
                {
                    "target": args.target,
                    "checked": checked,
                    "blocked": [
                        {"crate": crate, "build_script": script, "reason": reason}
                        for crate, script, reason in blocked
                    ],
                    "broken": [crate for crate, _ in broken],
                },
                indent=2,
                sort_keys=True,
            )
        )
    else:
        print(
            f"cross-check {args.target}: {len(checked)} checked, "
            f"{len(blocked)} blocked by an Apple toolchain"
        )
        if checked:
            print(f"  checked: {' '.join(checked)}")
        for crate, script, reason in blocked:
            print(f"  blocked: {crate} ({script} {reason})")

    for crate, output in broken:
        print(f"cross-check: {crate} failed to compile for {args.target}", file=sys.stderr)
        print(output, file=sys.stderr)

    if broken:
        return 1
    if REQUIRED_CROSS_CHECKED_CRATE in crates and REQUIRED_CROSS_CHECKED_CRATE not in checked:
        print(
            f"cross-check: {REQUIRED_CROSS_CHECKED_CRATE} must stay cross-checkable; "
            "it exists so the macOS system calls compile on this host. Keep its "
            "dependency closure free of build scripts that need an Apple toolchain.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
