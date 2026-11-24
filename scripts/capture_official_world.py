#!/usr/bin/env python3
"""Official 26.1 world capture manifest, identity, and receipt tooling.

Builds a deterministic manifest of an official Mojang dedicated-server world
save, derives a stable ``world_identity`` hash, and writes/verifies the frozen
capture receipt schema consumed by the stage3 evidence generator.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
VANILLA_VERSION = "26.1"
LEVEL_NAME = "world"
LEVEL_TYPE = "minecraft:flat"
DEFAULT_SEED = "bbb-official-capture-26.1"

HEX64 = re.compile(r"^[0-9a-f]{64}$")
CAPTURED_AT = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
REGION_FILE = re.compile(r"^r\.-?\d+\.-?\d+\.mca$")

RECEIPT_KEYS = {
    "schema_version",
    "vanilla_version",
    "captured_at",
    "world",
    "tool",
    "dimensions",
    "files",
    "world_identity",
}
WORLD_KEYS = {"level_name", "seed", "level_type"}
TOOL_KEYS = {"java_sha256", "server_jar_sha256"}
FILE_KEYS = {"path", "sha256", "size"}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def is_excluded(relative: str) -> bool:
    name = relative.rsplit("/", 1)[-1]
    return name == "session.lock" or name.endswith(".tmp")


def build_manifest(world_dir: Path) -> list[dict[str, Any]]:
    """Return the sorted manifest of every non-excluded regular file."""
    manifest = []
    for root, dirnames, filenames in os.walk(world_dir):
        dirnames.sort()
        for filename in sorted(filenames):
            path = Path(root) / filename
            if not path.is_file() or path.is_symlink():
                continue
            relative = path.relative_to(world_dir).as_posix()
            if is_excluded(relative):
                continue
            manifest.append(
                {
                    "path": relative,
                    "sha256": sha256_file(path),
                    "size": path.stat().st_size,
                }
            )
    manifest.sort(key=lambda entry: entry["path"])
    return manifest


def manifest_identity(manifest: list[dict[str, Any]]) -> str:
    canonical = json.dumps(manifest, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def dimension_dirs(world_dir: Path) -> list[str]:
    """Vanilla 26.1 stores every dimension under `dimensions/<ns>/<name>`
    (`DimensionType.getStorageFolder`)."""
    base = world_dir / "dimensions" / "minecraft"
    if not base.is_dir():
        return []
    return sorted(entry.name for entry in base.iterdir() if entry.is_dir())


def check_world_structure(
    world_dir: Path, manifest: list[dict[str, Any]]
) -> None:
    """Fail when the save is missing vanilla LevelStorageSource structure."""
    paths = {entry["path"] for entry in manifest}
    if "level.dat" not in paths:
        raise ValueError(f"world at {world_dir} is missing level.dat")
    overworld_region = "dimensions/minecraft/overworld/region/"
    if not any(
        path.startswith(overworld_region)
        and REGION_FILE.match(path[len(overworld_region):])
        for path in paths
    ):
        raise ValueError(
            f"world at {world_dir} has no {overworld_region}r.*.mca file"
        )
    if not any(
        path.startswith("players/data/") and path.endswith(".dat")
        for path in paths
    ):
        raise ValueError(f"world at {world_dir} has no players/data/*.dat file")
    if not (world_dir / "data").is_dir():
        raise ValueError(f"world at {world_dir} is missing the data/ directory")


def build_receipt(
    world_dir: Path,
    java_bin: Path,
    server_jar: Path,
    seed: str = DEFAULT_SEED,
    captured_at: str | None = None,
) -> dict[str, Any]:
    if captured_at is None:
        captured_at = (
            datetime.now(UTC)
            .replace(microsecond=0)
            .isoformat()
            .replace("+00:00", "Z")
        )
    manifest = build_manifest(world_dir)
    check_world_structure(world_dir, manifest)
    receipt = {
        "schema_version": SCHEMA_VERSION,
        "vanilla_version": VANILLA_VERSION,
        "captured_at": captured_at,
        "world": {
            "level_name": LEVEL_NAME,
            "seed": seed,
            "level_type": LEVEL_TYPE,
        },
        "tool": {
            "java_sha256": sha256_file(java_bin),
            "server_jar_sha256": sha256_file(server_jar),
        },
        "dimensions": dimension_dirs(world_dir),
        "files": manifest,
        "world_identity": manifest_identity(manifest),
    }
    validate_capture_receipt(receipt)
    return receipt


def _check_hash(value: Any, field: str) -> None:
    if not isinstance(value, str) or not HEX64.match(value):
        raise ValueError(f"receipt field {field!r} must be a 64-digit hex hash")


def validate_capture_receipt(value: Any) -> None:
    """Raise ValueError on any drift from the frozen receipt schema."""
    if not isinstance(value, dict):
        raise ValueError("receipt must be a JSON object")
    if set(value) != RECEIPT_KEYS:
        raise ValueError(
            f"receipt keys drifted: {sorted(set(value) ^ RECEIPT_KEYS)}"
        )
    if value["schema_version"] != SCHEMA_VERSION:
        raise ValueError("receipt schema_version must be 1")
    if value["vanilla_version"] != VANILLA_VERSION:
        raise ValueError(f"receipt vanilla_version must be {VANILLA_VERSION!r}")
    if not isinstance(value["captured_at"], str) or not CAPTURED_AT.match(
        value["captured_at"]
    ):
        raise ValueError(
            "receipt captured_at must be ISO-8601 UTC with seconds precision"
        )
    world = value["world"]
    if not isinstance(world, dict) or set(world) != WORLD_KEYS:
        raise ValueError("receipt world must have exactly level_name/seed/level_type")
    if world["level_name"] != LEVEL_NAME:
        raise ValueError(f"receipt world.level_name must be {LEVEL_NAME!r}")
    if not isinstance(world["seed"], str) or not world["seed"]:
        raise ValueError("receipt world.seed must be a non-empty string")
    if world["level_type"] != LEVEL_TYPE:
        raise ValueError(f"receipt world.level_type must be {LEVEL_TYPE!r}")
    tool = value["tool"]
    if not isinstance(tool, dict) or set(tool) != TOOL_KEYS:
        raise ValueError(
            "receipt tool must have exactly java_sha256/server_jar_sha256"
        )
    _check_hash(tool["java_sha256"], "tool.java_sha256")
    _check_hash(tool["server_jar_sha256"], "tool.server_jar_sha256")
    dimensions = value["dimensions"]
    if (
        not isinstance(dimensions, list)
        or not dimensions
        or any(not isinstance(name, str) or not name for name in dimensions)
        or dimensions != sorted(set(dimensions))
        or "overworld" not in dimensions
    ):
        raise ValueError(
            "receipt dimensions must be sorted unique names including 'overworld'"
        )
    files = value["files"]
    if not isinstance(files, list):
        raise ValueError("receipt files must be a list")
    previous = None
    for entry in files:
        if not isinstance(entry, dict) or set(entry) != FILE_KEYS:
            raise ValueError("receipt file entries must have exactly path/sha256/size")
        path = entry["path"]
        if (
            not isinstance(path, str)
            or not path
            or path.startswith("/")
            or "\\" in path
            or any(part in ("", ".", "..") for part in path.split("/"))
        ):
            raise ValueError(f"receipt file path {path!r} is not a clean relative path")
        _check_hash(entry["sha256"], f"files[{path}].sha256")
        size = entry["size"]
        if not isinstance(size, int) or isinstance(size, bool) or size < 0:
            raise ValueError(f"receipt file size for {path!r} must be a non-negative int")
        if previous is not None and path <= previous:
            raise ValueError("receipt files must be sorted with unique paths")
        previous = path
    _check_hash(value["world_identity"], "world_identity")
    if value["world_identity"] != manifest_identity(files):
        raise ValueError("receipt world_identity does not match its files manifest")


def atomic_write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    os.replace(tmp, path)


def load_receipt(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read receipt {path}: {error}") from error
    validate_capture_receipt(value)
    return value


def verify_world(world_dir: Path, receipt: dict[str, Any]) -> None:
    """Raise ValueError with a diff summary when the world drifts."""
    validate_capture_receipt(receipt)
    manifest = build_manifest(world_dir)
    identity = manifest_identity(manifest)
    if manifest == receipt["files"] and identity == receipt["world_identity"]:
        return
    expected = {entry["path"]: entry for entry in receipt["files"]}
    actual = {entry["path"]: entry for entry in manifest}
    missing = sorted(set(expected) - set(actual))
    extra = sorted(set(actual) - set(expected))
    changed = sorted(
        path
        for path in set(expected) & set(actual)
        if expected[path] != actual[path]
    )
    summary = (
        f"world at {world_dir} differs from receipt: "
        f"missing={missing[:5]} extra={extra[:5]} changed={changed[:5]} "
        f"(expected {len(expected)} files, got {len(actual)})"
    )
    raise ValueError(summary)


def command_capture(args: argparse.Namespace) -> None:
    receipt = build_receipt(
        world_dir=args.world_dir,
        java_bin=args.java_bin,
        server_jar=args.server_jar,
        seed=args.seed,
    )
    atomic_write_json(args.output, receipt)
    print(
        f"capture receipt: {args.output} "
        f"world_identity {receipt['world_identity']} "
        f"files={len(receipt['files'])}"
    )


def command_verify(args: argparse.Namespace) -> None:
    receipt = load_receipt(args.receipt)
    verify_world(args.world_dir, receipt)
    print(
        f"capture verified: {args.world_dir} matches {args.receipt} "
        f"world_identity {receipt['world_identity']}"
    )


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    subcommands = result.add_subparsers(dest="command", required=True)
    capture_parser = subcommands.add_parser("capture")
    capture_parser.add_argument("--world-dir", type=Path, required=True)
    capture_parser.add_argument("--java-bin", type=Path, required=True)
    capture_parser.add_argument("--server-jar", type=Path, required=True)
    capture_parser.add_argument("--seed", default=DEFAULT_SEED)
    capture_parser.add_argument("--output", type=Path, required=True)
    capture_parser.set_defaults(run=command_capture)
    verify_parser = subcommands.add_parser("verify")
    verify_parser.add_argument("--world-dir", type=Path, required=True)
    verify_parser.add_argument("--receipt", type=Path, required=True)
    verify_parser.set_defaults(run=command_verify)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        args.run(args)
    except (ValueError, OSError) as error:
        print(f"official capture failed: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
