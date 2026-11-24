"""Content-integrity verification with fast stat-stamp reuse for Vanilla workspaces."""

from __future__ import annotations

import hashlib
import json
import os
import stat
from collections.abc import Iterable
from pathlib import Path
from typing import Any

from .errors import IntegrityFailure
from .integrity import tree_hash, verify_file
from .lockfile import load_lock, lock_digest
from .workspace import VanillaWorkspace

_STAMP_SCHEMA = 1


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


def _stat_digest_downloads(
    workspace: VanillaWorkspace, lock: dict[str, Any]
) -> tuple[str, int, int] | None:
    hasher = hashlib.sha256()
    for artifact in locked_downloads(lock):
        target = workspace.downloads / artifact["path"]
        try:
            st = target.lstat()
        except OSError:
            return None
        if not stat.S_ISREG(st.st_mode) or st.st_size != artifact["size"]:
            return None
        hasher.update(f"{artifact['path']}\0{st.st_size}\0{st.st_mtime_ns}\n".encode())

    object_count = 0
    object_bytes = 0
    for artifact in asset_objects(workspace.downloads / lock["asset_index"]["path"], lock):
        target = workspace.downloads / artifact["path"]
        try:
            st = target.lstat()
        except OSError:
            return None
        if not stat.S_ISREG(st.st_mode) or st.st_size != artifact["size"]:
            return None
        hasher.update(f"{artifact['path']}\0{st.st_size}\0{st.st_mtime_ns}\n".encode())
        object_count += 1
        object_bytes += artifact["size"]

    expected_total = lock["asset_index"].get("total_size")
    if expected_total is not None and object_bytes != expected_total:
        return None
    return hasher.hexdigest(), object_count, object_bytes


def _stat_digest_tree(root: Path) -> str | None:
    if not root.is_dir() or root.is_symlink():
        return None
    hasher = hashlib.sha256()
    root_str = str(root)
    for current_dir, dirnames, filenames in os.walk(root_str, followlinks=False):
        dirnames.sort()
        filenames.sort()
        rel_dir = os.path.relpath(current_dir, root_str)
        rel_prefix = "" if rel_dir == "." else rel_dir.replace("\\", "/") + "/"
        for dirname in dirnames:
            candidate = os.path.join(current_dir, dirname)
            try:
                if stat.S_ISLNK(os.lstat(candidate).st_mode):
                    return None
            except OSError:
                return None
        for filename in filenames:
            candidate = os.path.join(current_dir, filename)
            try:
                st = os.lstat(candidate)
            except OSError:
                return None
            if not stat.S_ISREG(st.st_mode):
                return None
            hasher.update(
                f"{rel_prefix}{filename}\0{st.st_size}\0{st.st_mtime_ns}\n".encode()
            )
    return hasher.hexdigest()


def _write_stamp(
    workspace: VanillaWorkspace,
    *,
    expected_lock_digest: str,
    state_sha256: str,
    downloads_stat_sha256: str,
    sources_stat_sha256: str,
    source_identity: str,
) -> None:
    payload = {
        "schema": _STAMP_SCHEMA,
        "version": workspace.version,
        "lock_sha256": expected_lock_digest,
        "state_sha256": state_sha256,
        "downloads_stat_sha256": downloads_stat_sha256,
        "sources_stat_sha256": sources_stat_sha256,
        "sources": source_identity,
    }
    try:
        workspace.verify_stamp.parent.mkdir(parents=True, exist_ok=True)
        tmp = workspace.verify_stamp.with_name(
            f".{workspace.verify_stamp.name}.{os.getpid()}.tmp"
        )
        tmp.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        os.replace(tmp, workspace.verify_stamp)
    except OSError:
        # Read-only Vanilla mounts still succeed after full verification.
        pass


def verify_workspace(
    workspace: VanillaWorkspace,
    *,
    compare_root: Path | None = None,
    full: bool = False,
) -> dict[str, Any]:
    lock_path, lock = load_lock(workspace.version)
    state = workspace.load_state()
    expected_lock_digest = lock_digest(lock_path)
    if state.get("lock_sha256") != expected_lock_digest:
        raise IntegrityFailure(
            "Vanilla workspace was prepared from a different lock: "
            f"expected {expected_lock_digest}, got {state.get('lock_sha256')!r}"
        )

    state_sha256 = hashlib.sha256(workspace.state.read_bytes()).hexdigest()
    if not full and compare_root is None and workspace.verify_stamp.is_file():
        try:
            stamp = json.loads(workspace.verify_stamp.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            stamp = None
        if isinstance(stamp, dict) and (
            stamp.get("schema") == _STAMP_SCHEMA
            and stamp.get("version") == workspace.version
            and stamp.get("lock_sha256") == expected_lock_digest
            and stamp.get("state_sha256") == state_sha256
            and stamp.get("sources") == state.get("sources")
            and workspace.assets.is_dir()
        ):
            dl_stat = _stat_digest_downloads(workspace, lock)
            src_stat = _stat_digest_tree(workspace.sources)
            if (
                dl_stat is not None
                and src_stat is not None
                and dl_stat[0] == stamp.get("downloads_stat_sha256")
                and src_stat == stamp.get("sources_stat_sha256")
            ):
                return {
                    "status": "verified",
                    "version": workspace.version,
                    "root": str(workspace.root),
                    "lock_sha256": expected_lock_digest,
                    "sources": state["sources"],
                    "asset_objects": dl_stat[1],
                    "asset_object_bytes": dl_stat[2],
                    "comparison": None,
                }

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

    dl_stat = _stat_digest_downloads(workspace, lock)
    src_stat = _stat_digest_tree(workspace.sources)
    if dl_stat is not None and src_stat is not None:
        _write_stamp(
            workspace,
            expected_lock_digest=expected_lock_digest,
            state_sha256=state_sha256,
            downloads_stat_sha256=dl_stat[0],
            sources_stat_sha256=src_stat,
            source_identity=source_identity,
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
