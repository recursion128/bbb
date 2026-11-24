"""Content hashing and deterministic source-tree identity."""

from __future__ import annotations

import hashlib
import os
from pathlib import Path

from .errors import DigestMismatch, IntegrityFailure


def file_hash(path: Path, algorithm: str) -> str:
    digest = hashlib.new(algorithm)
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_file(
    path: Path,
    *,
    size: int,
    sha1: str,
    sha256: str | None = None,
    label: str | None = None,
) -> None:
    label = label or str(path)
    if not path.is_file():
        raise IntegrityFailure(f"missing {label}: {path}")
    actual_size = path.stat().st_size
    if actual_size != size:
        raise DigestMismatch(f"size mismatch for {label}: expected {size}, got {actual_size}")
    actual_sha1 = file_hash(path, "sha1")
    if actual_sha1 != sha1:
        raise DigestMismatch(f"SHA-1 mismatch for {label}: expected {sha1}, got {actual_sha1}")
    if sha256 is not None:
        actual_sha256 = file_hash(path, "sha256")
        if actual_sha256 != sha256:
            raise DigestMismatch(
                f"SHA-256 mismatch for {label}: expected {sha256}, got {actual_sha256}"
            )


def tree_hash(root: Path) -> dict[str, int | str]:
    if not root.is_dir():
        raise IntegrityFailure(f"source tree is missing: {root}")
    digest = hashlib.sha256()
    count = 0
    total = 0
    for directory, directory_names, file_names in os.walk(root):
        directory_names.sort()
        file_names.sort()
        directory_path = Path(directory)
        for name in file_names:
            path = directory_path / name
            if path.is_symlink():
                raise IntegrityFailure(f"source tree contains symlink: {path}")
            relative = path.relative_to(root).as_posix().encode("utf-8")
            size = path.stat().st_size
            digest.update(b"F\0")
            digest.update(str(len(relative)).encode("ascii"))
            digest.update(b"\0")
            digest.update(relative)
            digest.update(b"\0")
            digest.update(str(size).encode("ascii"))
            digest.update(b"\0")
            with path.open("rb") as source:
                for chunk in iter(lambda: source.read(1024 * 1024), b""):
                    digest.update(chunk)
            digest.update(b"\0")
            count += 1
            total += size
    return {"sha256": digest.hexdigest(), "files": count, "bytes": total}
