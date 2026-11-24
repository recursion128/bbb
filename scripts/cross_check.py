#!/usr/bin/env python3
"""Type-check workspace crates for a second desktop platform target.

On Linux/Windows hosts, code behind `cfg(target_os = "macos")` is not compiled
by the host check; on macOS hosts, Linux/Windows platform branches are not
compiled by the host check. `cross_check.py` picks an installed second-platform
target (or an explicit `--target`), excludes workspace crates whose target
dependency graph reaches known cross-target C/assembly build scripts (such as
`ring`) when cross-compiling across OSes, and runs a single `cargo check
--workspace --target <triple>`. If that fails, per-crate fallback attributes
the exact broken crate or build script.
"""

from __future__ import annotations

import argparse
import json
import os
import platform as py_platform
import re
import subprocess
import sys
from collections import deque
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from bbb_tools.python_runner import default_target_dir as shared_default_target_dir  # noqa: E402

DEFAULT_TARGET = "aarch64-apple-darwin"
DARWIN_CROSS_TARGETS = ("x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc")
NON_DARWIN_CROSS_TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
)

# Build scripts that require a target-native C/assembly cross-toolchain when
# checking a different OS triple than the host.
APPLE_TOOLCHAIN_BUILD_SCRIPTS = {
    "ring": "compiles C and assembly for the target",
}
CROSS_TOOLCHAIN_BUILD_SCRIPTS = APPLE_TOOLCHAIN_BUILD_SCRIPTS

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


def _toolchain_env() -> dict[str, str]:
    env = dict(os.environ)
    cargo_bin = env.get("CARGO", "").strip()
    if cargo_bin:
        cargo_parent = Path(cargo_bin).parent
        if str(cargo_parent) not in ("", "."):
            parent_str = str(cargo_parent)
            existing = env.get("PATH", "")
            parts = existing.split(os.pathsep) if existing else []
            if parent_str not in parts:
                env["PATH"] = f"{parent_str}{os.pathsep}{existing}" if existing else parent_str
    return env


def installed_targets() -> list[str]:
    result = subprocess.run(
        ["rustup", "target", "list", "--installed"],
        env=_toolchain_env(),
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if result.returncode != 0:
        return []
    return result.stdout.split()


def target_is_installed(target: str) -> bool:
    return target in installed_targets()


def host_triple() -> str:
    result = subprocess.run(
        ["rustc", "-vV"],
        env=_toolchain_env(),
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if result.returncode == 0:
        for line in result.stdout.splitlines():
            if line.startswith("host:"):
                host = line.split(":", 1)[1].strip()
                if host:
                    return host
    if sys.platform == "darwin":
        return (
            "aarch64-apple-darwin"
            if py_platform.machine().lower() in ("arm64", "aarch64")
            else "x86_64-apple-darwin"
        )
    if sys.platform == "win32":
        return "x86_64-pc-windows-msvc"
    return "x86_64-unknown-linux-gnu"


def candidate_cross_targets(host: str) -> list[str]:
    pool = DARWIN_CROSS_TARGETS if "apple-darwin" in host else NON_DARWIN_CROSS_TARGETS
    return [target for target in pool if target != host]


def select_cross_target(
    explicit_target: str | None,
    *,
    host: str | None = None,
    installed: list[str] | None = None,
) -> tuple[str | None, str]:
    installed_list = installed if installed is not None else installed_targets()
    if explicit_target is not None:
        if explicit_target in installed_list:
            return explicit_target, ""
        return (
            None,
            f"cross-check: skipped, rustup target {explicit_target} is not installed "
            f"(add it with: rustup target add {explicit_target})",
        )

    resolved_host = host if host is not None else host_triple()
    candidates = candidate_cross_targets(resolved_host)
    for candidate in candidates:
        if candidate in installed_list:
            return candidate, ""

    preferred = candidates[0] if candidates else DEFAULT_TARGET
    installed_summary = ", ".join(installed_list) if installed_list else "none"
    return (
        None,
        f"cross-check: skipped, no second-platform rustup target installed for host "
        f"{resolved_host} (installed: {installed_summary}; "
        f"add one with: rustup target add {preferred})",
    )


def classify_failure(cargo_output: str) -> tuple[str, str] | None:
    """The Apple-toolchain build script this failure stopped at, if any."""
    for match in BUILD_SCRIPT_FAILURE_RE.finditer(cargo_output):
        package = match.group(1)
        reason = APPLE_TOOLCHAIN_BUILD_SCRIPTS.get(package)
        if reason is not None:
            return package, reason
    return None


def default_target_dir() -> str:
    return str(shared_default_target_dir())


def _cargo_env() -> tuple[str, dict[str, str]]:
    env = _toolchain_env()
    env.setdefault("CARGO_TARGET_DIR", default_target_dir())
    cargo_bin = env.get("CARGO") or "cargo"
    return cargo_bin, env


def apple_blocked_from_metadata(
    metadata: dict[str, object], crates: list[str]
) -> dict[str, tuple[str, str]]:
    """Map workspace crate -> (build_script_pkg, reason) from cargo metadata."""
    packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    workspace_ids = set(metadata.get("workspace_members") or [])
    if not isinstance(packages, list) or not isinstance(resolve, dict):
        return {}
    nodes = resolve.get("nodes")
    if not isinstance(nodes, list):
        return {}

    id_to_name: dict[str, str] = {}
    member_name_to_id: dict[str, str] = {}
    for pkg in packages:
        if isinstance(pkg, dict) and isinstance(pkg.get("id"), str) and isinstance(pkg.get("name"), str):
            pkg_id = pkg["id"]
            pkg_name = pkg["name"]
            id_to_name[pkg_id] = pkg_name
            if pkg_id in workspace_ids:
                member_name_to_id[pkg_name] = pkg_id

    adjacency: dict[str, list[str]] = {}
    for node in nodes:
        if not isinstance(node, dict) or not isinstance(node.get("id"), str):
            continue
        edges: list[str] = []
        for dep in node.get("deps") or []:
            if not isinstance(dep, dict) or not isinstance(dep.get("pkg"), str):
                continue
            dep_kinds = dep.get("dep_kinds")
            if isinstance(dep_kinds, list) and dep_kinds:
                if not any(
                    isinstance(dk, dict) and dk.get("kind") in (None, "build")
                    for dk in dep_kinds
                ):
                    continue
            edges.append(dep["pkg"])
        adjacency[node["id"]] = edges

    blocked: dict[str, tuple[str, str]] = {}
    priority = list(APPLE_TOOLCHAIN_BUILD_SCRIPTS.keys())
    for crate in crates:
        start_id = member_name_to_id.get(crate)
        if start_id is None:
            continue
        seen = {start_id}
        queue = deque([start_id])
        reached_blockers: set[str] = set()
        while queue:
            current = queue.popleft()
            pkg_name = id_to_name.get(current, "")
            if pkg_name in APPLE_TOOLCHAIN_BUILD_SCRIPTS:
                reached_blockers.add(pkg_name)
            for nxt in adjacency.get(current, ()):
                if nxt not in seen:
                    seen.add(nxt)
                    queue.append(nxt)
        if reached_blockers:
            chosen = next(p for p in priority if p in reached_blockers)
            blocked[crate] = (chosen, APPLE_TOOLCHAIN_BUILD_SCRIPTS[chosen])
    return blocked


def target_os_family(triple: str) -> str:
    if "apple-darwin" in triple:
        return "darwin"
    if "windows" in triple:
        return "windows"
    if "linux" in triple:
        return "linux"
    return triple


def resolve_apple_blocked_crates(
    repo_root: Path, crates: list[str], target: str, *, host: str | None = None
) -> dict[str, tuple[str, str]]:
    resolved_host = host if host is not None else host_triple()
    if target_os_family(resolved_host) == target_os_family(target):
        return {}
    cargo_bin, env = _cargo_env()
    result = subprocess.run(
        [
            cargo_bin,
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--filter-platform",
            target,
        ],
        cwd=repo_root,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if result.returncode != 0:
        return {}
    try:
        metadata = json.loads(result.stdout)
    except json.JSONDecodeError:
        return {}
    return apple_blocked_from_metadata(metadata, crates)


def check_workspace_excluding(
    repo_root: Path, target: str, excluded: list[str]
) -> tuple[bool, str]:
    cargo_bin, env = _cargo_env()
    cmd = [cargo_bin, "check", "--quiet", "--locked", "--workspace", "--target", target]
    for crate in excluded:
        cmd.extend(["--exclude", crate])
    result = subprocess.run(
        cmd,
        cwd=repo_root,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return result.returncode == 0, result.stdout


def check_crate(repo_root: Path, crate: str, target: str) -> tuple[bool, str]:
    cargo_bin, env = _cargo_env()
    result = subprocess.run(
        [cargo_bin, "check", "--quiet", "--locked", "-p", crate, "--target", target],
        cwd=repo_root,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return result.returncode == 0, result.stdout


def run_partition(
    repo_root: Path, crates: list[str], target: str
) -> tuple[list[str], list[tuple[str, str, str]], list[tuple[str, str]]]:
    pre_blocked = resolve_apple_blocked_crates(repo_root, crates, target)
    candidates = [crate for crate in crates if crate not in pre_blocked]

    if candidates:
        ws_ok, _ws_out = check_workspace_excluding(
            repo_root, target, list(pre_blocked.keys())
        )
    else:
        ws_ok = True

    if ws_ok:
        checked = candidates
        blocked = [
            (crate, pre_blocked[crate][0], pre_blocked[crate][1])
            for crate in crates
            if crate in pre_blocked
        ]
        return checked, blocked, []

    checked = []
    blocked = []
    broken: list[tuple[str, str]] = []
    for crate in crates:
        if crate in pre_blocked:
            blocked.append((crate, pre_blocked[crate][0], pre_blocked[crate][1]))
            continue
        ok, output = check_crate(repo_root, crate, target)
        if ok:
            checked.append(crate)
            continue
        blocker = classify_failure(output)
        if blocker is None:
            broken.append((crate, output))
        else:
            blocked.append((crate, blocker[0], blocker[1]))
    return checked, blocked, broken


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", default=None)
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

    resolved_host = host_triple()
    target, skip_message = select_cross_target(args.target, host=resolved_host)
    if target is None:
        if args.json:
            print(
                json.dumps(
                    {
                        "host": resolved_host,
                        "target": None,
                        "skipped": True,
                        "reason": skip_message,
                        "checked": [],
                        "blocked": [],
                        "broken": [],
                    },
                    indent=2,
                    sort_keys=True,
                )
            )
        else:
            print(skip_message)
        return 0

    checked, blocked, broken = run_partition(args.repo_root, crates, target)

    if args.json:
        print(
            json.dumps(
                {
                    "host": resolved_host,
                    "target": target,
                    "skipped": False,
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
            f"cross-check {resolved_host} -> {target}: {len(checked)} checked, "
            f"{len(blocked)} blocked by cross-target native build scripts"
        )
        if checked:
            print(f"  checked: {' '.join(checked)}")
        for crate, script, reason in blocked:
            print(f"  blocked: {crate} ({script} {reason})")

    for crate, output in broken:
        print(f"cross-check: {crate} failed to compile for {target}", file=sys.stderr)
        print(output, file=sys.stderr)

    if broken:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
