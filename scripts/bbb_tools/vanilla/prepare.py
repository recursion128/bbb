"""Locked, atomic Minecraft distribution and source preparation."""

from __future__ import annotations

import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any

from .download import download_artifact
from .errors import (
    DecompileFailure,
    DigestMismatch,
    IntegrityFailure,
    JavaIncompatible,
    WorkspaceMissing,
)
from .integrity import file_hash, tree_hash, verify_file
from .lockfile import load_lock, lock_digest
from .locking import VersionLock
from .verify import asset_objects, locked_downloads, verify_workspace
from .workspace import VanillaWorkspace

_REPO_ROOT = Path(__file__).resolve().parents[3]


def _validate_cache_location(workspace: VanillaWorkspace) -> None:
    root = workspace.root.resolve()
    try:
        root.relative_to(_REPO_ROOT.resolve())
    except ValueError:
        pass
    else:
        raise IntegrityFailure(f"Vanilla cache must not be inside the bbb worktree: {root}")
    if (root / ".git").exists():
        raise IntegrityFailure(f"Vanilla cache must not be a Git repository: {root}")


def _java_major(java: str) -> tuple[int, str]:
    try:
        completed = subprocess.run([java, "-version"], capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise JavaIncompatible(f"cannot execute Java {java!r}: {error}") from error
    output = (completed.stderr or completed.stdout).strip()
    match = re.search(r'version "(?:1\.)?([0-9]+)', output)
    if completed.returncode != 0 or match is None:
        raise JavaIncompatible(f"cannot determine Java major from {java!r}: {output}")
    return int(match.group(1)), output.splitlines()[0]


def _valid_cached(path: Path, artifact: dict[str, Any]) -> bool:
    try:
        verify_file(
            path,
            size=artifact["size"],
            sha1=artifact["sha1"],
            sha256=artifact.get("sha256"),
            label=artifact["path"],
        )
        return True
    except (IntegrityFailure, DigestMismatch):
        return False


def _copy_or_download(
    artifact: dict[str, Any],
    *,
    current_downloads: Path,
    staged_downloads: Path,
    offline: bool,
) -> str:
    relative = Path(artifact["path"])
    source = current_downloads / relative
    destination = staged_downloads / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    if _valid_cached(source, artifact):
        try:
            os.link(source, destination)
        except OSError:
            shutil.copyfile(source, destination)
        return "cache-hit"
    if offline:
        raise WorkspaceMissing(
            f"offline Vanilla cache lacks {artifact['path']}; rerun prepare without --offline"
        )
    download_artifact(
        artifact["url"],
        destination,
        size=artifact["size"],
        sha1=artifact["sha1"],
        sha256=artifact.get("sha256"),
    )
    return "downloaded"


def _run_decompiler(
    lock: dict[str, Any],
    staged_downloads: Path,
    staged_sources: Path,
    *,
    java: str,
    jobs: int,
    log_path: Path,
) -> list[str]:
    arguments = [
        str(argument).replace("{threads}", str(jobs)) for argument in lock["decompile"]["arguments"]
    ]
    arguments.extend(
        f"--add-external={staged_downloads / library['path']}" for library in lock["libraries"]
    )
    command = [
        java,
        "-jar",
        str(staged_downloads / lock["vineflower"]["path"]),
        *arguments,
        str(staged_downloads / lock["client"]["path"]),
        str(staged_sources),
    ]
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("wb") as log:
        try:
            completed = subprocess.run(
                command,
                stdout=log,
                stderr=subprocess.STDOUT,
                timeout=lock["decompile"].get("timeout_seconds", 1800),
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise DecompileFailure(f"Vineflower failed to run ({error}); see {log_path}") from error
    if completed.returncode != 0:
        raise DecompileFailure(f"Vineflower exited with {completed.returncode}; see {log_path}")
    return command


def _remove_path(path: Path) -> None:
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    else:
        path.unlink(missing_ok=True)


def _publish(
    workspace: VanillaWorkspace,
    *,
    staged_downloads: Path,
    staged_sources: Path,
    staged_state: Path,
    transaction: Path,
) -> None:
    workspace.downloads.parent.mkdir(parents=True, exist_ok=True)
    workspace.sources.parent.mkdir(parents=True, exist_ok=True)
    workspace.state.parent.mkdir(parents=True, exist_ok=True)
    backup = transaction / "backup"
    backup.mkdir()
    targets = (
        (workspace.downloads, backup / "downloads"),
        (workspace.sources, backup / "sources"),
        (workspace.state, backup / "state.json"),
    )
    installed: list[Path] = []
    moved: list[tuple[Path, Path]] = []
    try:
        for target, saved in targets:
            if target.exists() or target.is_symlink():
                os.replace(target, saved)
                moved.append((target, saved))
        os.replace(staged_downloads, workspace.downloads)
        installed.append(workspace.downloads)
        os.replace(staged_sources, workspace.sources)
        installed.append(workspace.sources)
        os.replace(staged_state, workspace.state)
        installed.append(workspace.state)
    except BaseException:
        for path in reversed(installed):
            _remove_path(path)
        for target, saved in reversed(moved):
            if saved.exists() or saved.is_symlink():
                os.replace(saved, target)
        raise


def prepare_workspace(
    workspace: VanillaWorkspace,
    *,
    offline: bool = False,
    java: str = "java",
    jobs: int | None = None,
    wait_for_lock: bool = False,
) -> dict[str, Any]:
    _validate_cache_location(workspace)
    lock_path, lock = load_lock(workspace.version)
    expected_lock_digest = lock_digest(lock_path)
    try:
        return {"cache": "verified", **verify_workspace(workspace)}
    except (IntegrityFailure, WorkspaceMissing, DigestMismatch):
        pass
    jobs = jobs or os.cpu_count() or 1
    jobs = max(1, min(jobs, int(lock["decompile"].get("max_threads", 32))))
    resolved_java = shutil.which(java) if not Path(java).is_absolute() else java
    if resolved_java is None:
        raise JavaIncompatible(f"Java executable is unavailable: {java!r}")
    java = str(Path(resolved_java).expanduser().absolute())
    java_major, java_version = _java_major(java)
    required_java = lock["java"]["major"]
    if java_major != required_java:
        raise JavaIncompatible(
            f"Minecraft {workspace.version} requires Java major {required_java}; "
            f"found {java_major} at {java!r} ({java_version})"
        )
    with VersionLock(workspace.lock_file, wait=wait_for_lock):
        try:
            return {"cache": "verified-after-wait", **verify_workspace(workspace)}
        except (IntegrityFailure, WorkspaceMissing, DigestMismatch):
            pass
        transaction = workspace.staging / f"{workspace.version}-{uuid.uuid4().hex}"
        staged_downloads = transaction / "downloads"
        staged_sources = transaction / "sources"
        staged_state = transaction / "state.json"
        log_path = (
            workspace.root
            / "logs"
            / workspace.version
            / f"prepare-{int(time.time())}-{os.getpid()}.log"
        )
        transaction.mkdir(parents=True)
        started = int(time.time())
        receipts: dict[str, str] = {}
        try:
            initial = locked_downloads(lock)
            # The asset index is needed before its content-addressed objects can
            # be scheduled, so prepare the locked distribution in two stages.
            with ThreadPoolExecutor(max_workers=jobs) as pool:
                future_artifacts = {
                    pool.submit(
                        _copy_or_download,
                        artifact,
                        current_downloads=workspace.downloads,
                        staged_downloads=staged_downloads,
                        offline=offline,
                    ): artifact
                    for artifact in initial
                }
                for future in as_completed(future_artifacts):
                    artifact = future_artifacts[future]
                    receipts[artifact["path"]] = future.result()
            objects = list(asset_objects(staged_downloads / lock["asset_index"]["path"], lock))
            with ThreadPoolExecutor(max_workers=jobs) as pool:
                future_artifacts = {
                    pool.submit(
                        _copy_or_download,
                        artifact,
                        current_downloads=workspace.downloads,
                        staged_downloads=staged_downloads,
                        offline=offline,
                    ): artifact
                    for artifact in objects
                }
                for future in as_completed(future_artifacts):
                    artifact = future_artifacts[future]
                    receipts[artifact["path"]] = future.result()
            command = _run_decompiler(
                lock,
                staged_downloads,
                staged_sources,
                java=java,
                jobs=jobs,
                log_path=log_path,
            )
            if not (staged_sources / "assets" / "minecraft").is_dir():
                raise IntegrityFailure("Vineflower completed without extracting assets/minecraft")
            source_identity = tree_hash(staged_sources)
            state = {
                "schema_version": 1,
                "status": "complete",
                "version": workspace.version,
                "lock_sha256": expected_lock_digest,
                "started_unix": started,
                "completed_unix": int(time.time()),
                "host": {
                    "platform": sys.platform,
                    "machine": platform.machine(),
                    "python": platform.python_version(),
                },
                "java": {
                    "executable": java,
                    "major": java_major,
                    "version": java_version,
                },
                "vineflower": {
                    "sha256": file_hash(staged_downloads / lock["vineflower"]["path"], "sha256"),
                    "arguments": command[3:-2],
                    "jobs": jobs,
                    "log": str(log_path),
                },
                "downloads": {
                    "files": len(receipts),
                    "cache_hits": sum(value == "cache-hit" for value in receipts.values()),
                    "downloaded": sum(value == "downloaded" for value in receipts.values()),
                    "asset_objects": len(objects),
                },
                "sources": source_identity,
            }
            staged_state.write_text(
                json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8"
            )
            _publish(
                workspace,
                staged_downloads=staged_downloads,
                staged_sources=staged_sources,
                staged_state=staged_state,
                transaction=transaction,
            )
        finally:
            shutil.rmtree(transaction, ignore_errors=True)
    return {"cache": "prepared", **verify_workspace(workspace)}
