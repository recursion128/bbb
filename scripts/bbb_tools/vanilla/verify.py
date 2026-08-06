"""Full content-integrity verification for managed Vanilla workspaces."""

from __future__ import annotations

import json
from collections.abc import Iterable
from pathlib import Path
from typing import Any

from .errors import IntegrityFailure
from .integrity import tree_hash, verify_file
from .lockfile import load_lock, lock_digest
from .workspace import VanillaWorkspace


def locked_downloads(lock: dict[str, Any]) -> list[dict[str, Any]]:
    artifacts = [
        lock["metadata"],
        lock["client"],
        lock["server"],
        lock["asset_index"],
        lock["vineflower"],
        *lock["libraries"],
    ]
    return sorted(artifacts, key=lambda item: item["path"])


def asset_objects(index_path: Path, lock: dict[str, Any]) -> Iterable[dict[str, Any]]:
    try:
        index = json.loads(index_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise IntegrityFailure(f"cannot read asset index {index_path}: {error}") from error
    objects = index.get("objects") if isinstance(index, dict) else None
    if not isinstance(objects, dict):
        raise IntegrityFailure(f"asset index lacks an objects map: {index_path}")
    base_url = lock.get("assets", {}).get(
        "object_base_url", "https://resources.download.minecraft.net"
    )
    for logical_name, value in sorted(objects.items()):
        if (
            not isinstance(logical_name, str)
            or not isinstance(value, dict)
            or not isinstance(value.get("hash"), str)
            or len(value["hash"]) != 40
            or not isinstance(value.get("size"), int)
        ):
            raise IntegrityFailure(f"invalid asset-index entry {logical_name!r}")
        digest = value["hash"]
        yield {
            "logical_name": logical_name,
            "path": f"assets/objects/{digest[:2]}/{digest}",
            "url": f"{base_url}/{digest[:2]}/{digest}",
            "size": value["size"],
            "sha1": digest,
        }


def verify_workspace(
    workspace: VanillaWorkspace,
    *,
    compare_root: Path | None = None,
) -> dict[str, Any]:
    lock_path, lock = load_lock(workspace.version)
    state = workspace.load_state()
    expected_lock_digest = lock_digest(lock_path)
    if state.get("lock_sha256") != expected_lock_digest:
        raise IntegrityFailure(
            "Vanilla workspace was prepared from a different lock: "
            f"expected {expected_lock_digest}, got {state.get('lock_sha256')!r}"
        )
    for artifact in locked_downloads(lock):
        verify_file(
            workspace.downloads / artifact["path"],
            size=artifact["size"],
            sha1=artifact["sha1"],
            sha256=artifact.get("sha256"),
            label=artifact["path"],
        )
    object_count = 0
    object_bytes = 0
    for artifact in asset_objects(workspace.downloads / lock["asset_index"]["path"], lock):
        verify_file(
            workspace.downloads / artifact["path"],
            size=artifact["size"],
            sha1=artifact["sha1"],
            label=f"asset {artifact['logical_name']}",
        )
        object_count += 1
        object_bytes += artifact["size"]
    expected_total = lock["asset_index"].get("total_size")
    if expected_total is not None and object_bytes != expected_total:
        raise IntegrityFailure(
            f"asset object bytes mismatch: expected {expected_total}, got {object_bytes}"
        )
    source_identity = tree_hash(workspace.sources)
    if source_identity != state.get("sources"):
        raise IntegrityFailure(
            f"Vanilla source tree differs from state: expected "
            f"{state.get('sources')!r}, got {source_identity!r}"
        )
    if not workspace.assets.is_dir():
        raise IntegrityFailure(
            f"Vineflower output lacks extracted Minecraft assets: {workspace.assets}"
        )
    comparison = None
    if compare_root is not None:
        old_sources = compare_root.expanduser().absolute() / "sources" / workspace.version
        old_identity = tree_hash(old_sources)
        comparison = {
            "root": str(compare_root.expanduser().absolute()),
            "sources": old_identity,
            "equal": old_identity == source_identity,
        }
        if not comparison["equal"]:
            raise IntegrityFailure(
                "Vanilla source tree comparison failed: "
                f"managed={source_identity}, comparison={old_identity}"
            )
    return {
        "status": "verified",
        "version": workspace.version,
        "root": str(workspace.root),
        "lock_sha256": expected_lock_digest,
        "sources": source_identity,
        "asset_objects": object_count,
        "asset_object_bytes": object_bytes,
        "comparison": comparison,
    }
