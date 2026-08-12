"""Build the committed direct Cargo dependency modernization receipt."""

from __future__ import annotations

from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib


SCHEMA = 1
AUDIT_AS_OF = "2026-08-12"
CRATES_IO_API = "https://crates.io/api/v1/crates/{crate}"
RUSTSEC_DATABASE_COMMIT = "d0861df1eab469d3c58d6b836ce48b5766e5f217"

# Snapshot of crates.io's max_stable_version and that release's rust_version.
# Prereleases and yanked releases are deliberately excluded.
LATEST = {
    "al-sys": ("0.6.1", None),
    "anyhow": ("1.0.104", "1.68"),
    "arboard": ("3.6.1", "1.71.0"),
    "arc-swap": ("1.9.2", None),
    "base64": ("0.23.1", "1.71.0"),
    "bytemuck": ("1.25.2", None),
    "bytes": ("1.12.1", "1.57"),
    "chrono": ("0.4.45", "1.62.0"),
    "chrono-tz": ("0.10.4", "1.65"),
    "clap": ("4.6.6", "1.85"),
    "criterion": ("0.8.2", "1.86"),
    "flate2": ("1.1.9", "1.67.0"),
    "glam": ("0.33.3", "1.68.2"),
    "hecs": ("0.11.1", "1.81"),
    "hexf-parse": ("0.2.1", None),
    "icu_calendar": ("2.2.1", "1.86"),
    "icu_datetime": ("2.2.0", "1.86"),
    "icu_decimal": ("2.2.0", "1.86"),
    "icu_locale": ("2.2.0", "1.86"),
    "icu_time": ("2.2.0", "1.86"),
    "image": ("0.25.10", "1.88.0"),
    "isocountry": ("0.3.2", None),
    "kira": ("0.12.3", None),
    "libc": ("0.2.189", "1.65"),
    "md5": ("0.8.1", None),
    "num-bigint": ("0.5.1", "1.60"),
    "objc2": ("0.6.4", "1.71"),
    "objc2-app-kit": ("0.3.2", "1.71"),
    "objc2-foundation": ("0.3.2", "1.71"),
    "pollster": ("1.0.1", "1.69"),
    "rand": ("0.10.2", "1.85"),
    "regex": ("1.13.1", "1.65"),
    "reqwest": ("0.13.4", "1.85.0"),
    "rsa": ("0.9.10", "1.65"),
    "rustls": ("0.23.41", "1.71"),
    "ryu": ("1.0.23", "1.71"),
    "serde": ("1.0.229", "1.56"),
    "serde_json": ("1.0.151", "1.71"),
    "sha1": ("0.11.0", "1.85"),
    "sha2": ("0.11.0", "1.85"),
    "symphonia": ("0.6.0", "1.85"),
    "sys-locale": ("0.3.2", "1.56"),
    "thiserror": ("2.0.20", "1.71"),
    "tokio": ("1.53.1", "1.71"),
    "tracing": ("0.1.44", "1.65.0"),
    "tracing-subscriber": ("0.3.23", "1.65.0"),
    "unicode-bidi": ("0.3.18", "1.47.0"),
    "unicode-character-database": ("0.1.0", "1.17"),
    "unicode_names2": ("3.1.0", "1.65.0"),
    "url": ("2.5.8", "1.63"),
    "uuid": ("1.24.0", "1.85.0"),
    "wgpu": ("30.0.0", "1.87.0"),
    "winit": ("0.30.13", "1.70.0"),
    "writeable": ("0.6.3", "1.82"),
    "x11-dl": ("2.21.0", None),
    "zip": ("8.6.0", "1.88"),
}

LOW_ACTIVITY = {
    "al-sys": "latest release is from 2019 and it pulls the unmaintained rental crate",
    "hexf-parse": "latest release is from 2021; retain while the tiny parser API stays sufficient",
    "isocountry": "latest release is from 2020; retain while the locale-country mapping stays sufficient",
    "unicode-character-database": "latest release is from 2022; pinned Unicode table remains intentional",
    "x11-dl": "latest release is from 2023; X11 remains a reachable non-release implementation detail",
}

GROUPS = {
    "winit": "window-lifecycle",
    "pollster": "bootstrap-blocking",
    "reqwest": "authentication-network",
    "sha1": "authentication-network",
    "sha2": "authentication-network",
    "num-bigint": "authentication-network",
    "md5": "authentication-network",
    "rsa": "authentication-network",
    "rustls": "authentication-network",
    "glam": "performance-glam",
    "hecs": "performance-hecs",
    "symphonia": "audio-decoder",
    "zip": "archive",
    "criterion": "development-benchmark",
    "rand": "development-random",
}

PLANNED = frozenset(GROUPS) - {"rsa", "sha1", "sha2"}

ADVISORIES = {
    "al-sys": [
        {
            "id": "RUSTSEC-2021-0134",
            "kind": "transitive-unmaintained",
            "package": "rental 0.5.6",
            "reachability": "optional device audio backend",
        }
    ],
    "anyhow": [
        {
            "id": "RUSTSEC-2026-0190",
            "kind": "direct-unsound",
            "package": "anyhow 1.0.102",
            "patched": ">=1.0.103",
            "reachability": "affected downcast_mut API is not used, but lock refresh is required",
        }
    ],
    "criterion": [
        {
            "id": "RUSTSEC-2026-0204",
            "kind": "transitive-vulnerability",
            "package": "crossbeam-epoch 0.9.18",
            "patched": ">=0.9.20",
            "reachability": "development benchmarks only",
        }
    ],
    "hecs": [
        {
            "id": "crates.io-yanked",
            "kind": "transitive-yanked",
            "package": "spin 0.9.8",
            "reachability": "runtime ECS dependency",
        }
    ],
    "rsa": [
        {
            "id": "RUSTSEC-2023-0071",
            "kind": "direct-vulnerability",
            "package": "rsa 0.9.10",
            "patched": None,
            "reachability": "production uses public-key verification; private-key operations are test fixtures",
        },
        {
            "id": "crates.io-yanked",
            "kind": "transitive-yanked",
            "package": "spin 0.9.8 via num-bigint-dig",
            "reachability": "runtime public-key parsing and verification",
        },
    ],
    "symphonia": [
        {
            "id": "crates.io-yanked",
            "kind": "transitive-yanked",
            "package": "spin 0.9.8",
            "reachability": "runtime Ogg/Vorbis decoding",
        }
    ],
    "winit": [
        {
            "id": "RUSTSEC-2026-0194",
            "kind": "linux-transitive-vulnerability",
            "package": "quick-xml 0.39.3",
            "patched": ">=0.41.0",
            "reachability": "Linux Wayland build chain",
        },
        {
            "id": "RUSTSEC-2026-0195",
            "kind": "linux-transitive-vulnerability",
            "package": "quick-xml 0.39.3",
            "patched": ">=0.41.0",
            "reachability": "Linux Wayland build chain",
        },
        {
            "id": "RUSTSEC-2026-0186",
            "kind": "linux-transitive-unsound",
            "package": "memmap2 0.9.10",
            "patched": ">=0.9.11",
            "reachability": "Linux window backend",
        },
        {
            "id": "RUSTSEC-2026-0192",
            "kind": "linux-transitive-unmaintained",
            "package": "ttf-parser 0.25.1",
            "reachability": "Linux client-side decorations",
        },
    ],
}

MEMBER_DOMAINS = {
    "bbb-audio": {"audio", "performance"},
    "bbb-control": {"performance"},
    "bbb-item-model": {"authentication", "protocol", "rendering"},
    "bbb-native": {"audio", "authentication", "performance", "platform", "rendering"},
    "bbb-net": {"authentication", "protocol"},
    "bbb-pack": {"authentication", "protocol", "rendering"},
    "bbb-platform-macos": {"platform"},
    "bbb-protocol": {"authentication", "protocol"},
    "bbb-render-types": {"rendering"},
    "bbb-renderer": {"performance", "platform", "rendering"},
    "bbb-world": {"authentication", "performance", "protocol", "rendering"},
}


class DependencyReceiptError(RuntimeError):
    """The workspace no longer matches the audited dependency model."""


def _sha256(path: Path) -> str:
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def _run(root: Path, *args: str) -> str:
    return subprocess.run(
        args,
        cwd=root,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout


def _direct_manifest_entries(root: Path) -> dict[str, dict[str, object]]:
    root_manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    entries: dict[str, dict[str, object]] = {}
    for name, declaration in root_manifest["workspace"]["dependencies"].items():
        entries[name] = {"declaration": declaration, "manifest": "Cargo.toml"}

    for path in sorted((root / "crates").glob("*/Cargo.toml")):
        manifest = tomllib.loads(path.read_text(encoding="utf-8"))
        sections = [manifest]
        sections.extend(manifest.get("target", {}).values())
        for section in sections:
            for key in ("dependencies", "dev-dependencies", "build-dependencies"):
                for name, declaration in section.get(key, {}).items():
                    if isinstance(declaration, dict) and (
                        declaration.get("workspace") or "path" in declaration
                    ):
                        continue
                    if not isinstance(declaration, (str, dict)):
                        continue
                    relative = path.relative_to(root).as_posix()
                    current = entries.setdefault(
                        name, {"declaration": declaration, "manifest": relative}
                    )
                    if current["declaration"] != declaration:
                        raise DependencyReceiptError(
                            f"direct dependency {name} has inconsistent local declarations"
                        )
    return entries


def _constraint(declaration: object) -> str:
    if isinstance(declaration, str):
        return declaration
    if isinstance(declaration, dict) and isinstance(declaration.get("version"), str):
        return declaration["version"]
    raise DependencyReceiptError(f"direct registry dependency lacks a version: {declaration!r}")


def _metadata(root: Path) -> dict[str, object]:
    return json.loads(_run(root, "cargo", "metadata", "--format-version", "1", "--locked"))


def _lock_checksums(root: Path) -> dict[tuple[str, str, str], str]:
    packages = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))["package"]
    return {
        (package["name"], package["version"], package["source"]): package["checksum"]
        for package in packages
        if package.get("source", "").startswith("registry+") and package.get("checksum")
    }


def _resolved_direct_packages(
    metadata: dict[str, object],
) -> tuple[dict[str, set[tuple[str, str | None, str | None]]], dict[str, dict[str, set[str]]]]:
    packages = {package["id"]: package for package in metadata["packages"]}
    workspace_ids = set(metadata["workspace_members"])
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    resolved: dict[str, set[tuple[str, str | None, str | None]]] = defaultdict(set)
    usage: dict[str, dict[str, set[str]]] = defaultdict(lambda: defaultdict(set))

    for workspace_id in workspace_ids:
        package = packages[workspace_id]
        direct_by_name = {}
        for dependency in package["dependencies"]:
            direct_by_name[dependency["name"]] = dependency
            direct_by_name[(dependency.get("rename") or dependency["name"]).replace("-", "_")] = (
                dependency
            )
        for edge in nodes[workspace_id]["deps"]:
            target = packages[edge["pkg"]]
            if target.get("source") is None:
                continue
            dependency = direct_by_name.get(edge["name"])
            if dependency is None:
                raise DependencyReceiptError(
                    f"metadata edge {package['name']} -> {edge['name']} lacks a declaration"
                )
            name = target["name"]
            resolved[name].add((target["version"], target.get("source"), target.get("checksum")))
            usage[name]["members"].add(package["name"])
            for kind in edge["dep_kinds"]:
                usage[name]["classes"].add(kind["kind"] or "runtime")
                if kind.get("target"):
                    usage[name]["targets"].add(kind["target"])
            if dependency.get("optional"):
                usage[name]["classes"].add("optional")
    return resolved, usage


def _history(root: Path, name: str, manifest: str) -> dict[str, str]:
    pattern = rf"^{re.escape(name)}[[:space:]]*="
    commits = _run(root, "git", "log", "--format=%H", "-G", pattern, "--", manifest).split()
    if not commits:
        raise DependencyReceiptError(f"cannot locate manifest history for {name} in {manifest}")
    return {"first_introduced_commit": commits[-1], "latest_constraint_commit": commits[0]}


def _platforms(targets: set[str]) -> list[str]:
    if not targets:
        return ["linux", "macos"]
    result: set[str] = set()
    for target in targets:
        if "target_os = \"linux\"" in target:
            result.add("linux")
        elif "target_os = \"macos\"" in target:
            result.add("macos")
        else:
            result.update(("linux", "macos"))
    return sorted(result)


def _version_tuple(version: str) -> tuple[int, int, int]:
    match = re.match(r"^(\d+)(?:\.(\d+))?(?:\.(\d+))?", version)
    if match is None:
        raise DependencyReceiptError(f"unsupported version: {version}")
    return tuple(int(part or 0) for part in match.groups())  # type: ignore[return-value]


def _breaking_risk(constraint: str, latest: str) -> str:
    current = _version_tuple(constraint.lstrip("=~^<> "))
    target = _version_tuple(latest)
    if current == target:
        return "none"
    if current[0] != target[0] or (current[0] == 0 and current[1] != target[1]):
        return "high-major-or-pre-1.0-minor"
    return "low-compatible-line"


def _decision(name: str, locked: str, latest: str, constraint: str) -> tuple[str, str]:
    if name == "rsa":
        return (
            "retain-intentional",
            "0.9.10 is the latest stable release; the unpatched timing advisory does not cover "
            "BBB's production public-key verification path",
        )
    if name in {"sha1", "sha2"}:
        return (
            "retain-intentional",
            "0.11 uses digest 0.11 while the latest stable rsa 0.9.10 requires digest 0.10; "
            "reopen when a stable rsa release supports digest 0.11",
        )
    if name in {"reqwest", "rustls"} and locked == latest:
        return (
            "upgraded",
            "reqwest 0.13 uses rustls-no-provider with BBB-installed ring; this keeps system "
            "certificate verification without the aws-lc C cross-toolchain dependency",
        )
    if name in PLANNED:
        if locked == latest:
            return "upgraded", f"bounded {GROUPS[name]} migration is resolved at latest stable"
        return "upgrade-planned", f"bounded {GROUPS[name]} migration with focused parity tests"
    if locked == latest:
        if name in LOW_ACTIVITY:
            return "retain-low-activity", LOW_ACTIVITY[name]
        return "current", "lockfile already resolves the audited latest stable release"
    if _breaking_risk(constraint, latest) == "low-compatible-line":
        return "compatible-lock-refresh", "manifest accepts the release; refresh in the lock-only slice"
    return (
        "retain-intentional",
        "not in an approved migration group; reopen for an advisory, required API, or platform contract",
    )


def build_receipt(root: Path) -> dict[str, object]:
    entries = _direct_manifest_entries(root)
    if set(entries) != set(LATEST):
        missing = sorted(set(entries) - set(LATEST))
        stale = sorted(set(LATEST) - set(entries))
        raise DependencyReceiptError(f"latest-release audit is stale: missing={missing}, removed={stale}")

    metadata = _metadata(root)
    resolved, usage = _resolved_direct_packages(metadata)
    lock_checksums = _lock_checksums(root)
    dependencies = []
    for name in sorted(entries):
        matches = resolved.get(name, set())
        if len(matches) != 1:
            raise DependencyReceiptError(
                f"direct dependency {name} must resolve once, got {sorted(matches)}"
            )
        locked, source, _metadata_checksum = next(iter(matches))
        checksum = lock_checksums.get((name, locked, source or ""))
        if source != "registry+https://github.com/rust-lang/crates.io-index" or not checksum:
            raise DependencyReceiptError(f"direct dependency {name} is not registry checksummed")
        declaration = entries[name]["declaration"]
        constraint = _constraint(declaration)
        latest, latest_msrv = LATEST[name]
        members = sorted(usage[name]["members"])
        domains = sorted(set().union(*(MEMBER_DOMAINS[member] for member in members)))
        decision, rationale = _decision(name, locked, latest, constraint)
        dependencies.append(
            {
                "name": name,
                "manifest": entries[name]["manifest"],
                "manifest_constraint": constraint,
                "locked": {"version": locked, "source": source, "checksum": checksum},
                "latest_stable": {
                    "version": latest,
                    "rust_version": latest_msrv,
                    "source": CRATES_IO_API.format(crate=name),
                },
                **_history(root, name, str(entries[name]["manifest"])),
                "dependency_classes": sorted(usage[name]["classes"]),
                "workspace_members": members,
                "platform_impact": _platforms(usage[name]["targets"]),
                "domain_impact": domains,
                "maintenance": (
                    {"state": "latest-stable-low-activity", "note": LOW_ACTIVITY[name]}
                    if name in LOW_ACTIVITY
                    else {"state": "active-or-no-advisory"}
                ),
                "advisories": ADVISORIES.get(name, []),
                "migration_risk": {
                    "breaking_api": _breaking_risk(constraint, latest),
                    "workspace_msrv": "1.97",
                    "latest_release_msrv": latest_msrv,
                },
                "modernization": {
                    "group": GROUPS.get(name, "compatible-or-retained"),
                    "decision": decision,
                    "rationale": rationale,
                },
            }
        )

    return {
        "schema": SCHEMA,
        "audit_as_of": AUDIT_AS_OF,
        "inputs": {
            "Cargo.toml_sha256": _sha256(root / "Cargo.toml"),
            "Cargo.lock_sha256": _sha256(root / "Cargo.lock"),
            "cargo_metadata": "cargo metadata --format-version 1 --locked",
            "latest_release_rule": "crates.io max_stable_version; prerelease and yanked excluded",
            "rustsec_database_commit": RUSTSEC_DATABASE_COMMIT,
            "rustsec_command": "cargo audit 0.22.2 audit --json",
        },
        "workspace_msrv": "1.97",
        "direct_registry_dependency_count": len(dependencies),
        "dependencies": dependencies,
    }


def render_receipt(root: Path) -> str:
    return json.dumps(build_receipt(root), indent=2, ensure_ascii=False) + "\n"
