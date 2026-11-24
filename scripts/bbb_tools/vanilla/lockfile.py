"""Committed Vanilla supply-chain lock schema."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Any

from .errors import IntegrityFailure, VanillaError

LOCK_SCHEMA = 1
_SHA1 = re.compile(r"[0-9a-f]{40}\Z")
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
_SCRIPTS = Path(__file__).resolve().parents[2]


def lock_path(version: str) -> Path:
    return _SCRIPTS / "data" / f"vanilla-{version}.lock.json"


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def lock_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_lock(version: str, path: Path | None = None) -> tuple[Path, dict[str, Any]]:
    path = lock_path(version) if path is None else path
    if not path.is_file():
        raise VanillaError(f"no committed Vanilla lock for {version}: {path}")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise IntegrityFailure(f"cannot read Vanilla lock {path}: {error}") from error
    validate_lock(value, expected_version=version)
    return path, value


def _artifact(value: object, label: str, *, require_sha256: bool = False) -> None:
    if not isinstance(value, dict):
        raise IntegrityFailure(f"Vanilla lock {label} must be an object")
    for field in ("path", "url", "size", "sha1"):
        if field not in value:
            raise IntegrityFailure(f"Vanilla lock {label} lacks {field}")
    path = value["path"]
    if (
        not isinstance(path, str)
        or not path
        or Path(path).is_absolute()
        or ".." in Path(path).parts
    ):
        raise IntegrityFailure(f"Vanilla lock {label} has unsafe path {path!r}")
    if not isinstance(value["url"], str) or not value["url"].startswith("https://"):
        raise IntegrityFailure(f"Vanilla lock {label} URL must use HTTPS")
    if not isinstance(value["size"], int) or value["size"] < 0:
        raise IntegrityFailure(f"Vanilla lock {label} has invalid size")
    if not isinstance(value["sha1"], str) or _SHA1.fullmatch(value["sha1"]) is None:
        raise IntegrityFailure(f"Vanilla lock {label} has invalid SHA-1")
    sha256 = value.get("sha256")
    if require_sha256 and (not isinstance(sha256, str) or _SHA256.fullmatch(sha256) is None):
        raise IntegrityFailure(f"Vanilla lock {label} has invalid SHA-256")
    if sha256 is not None and (not isinstance(sha256, str) or _SHA256.fullmatch(sha256) is None):
        raise IntegrityFailure(f"Vanilla lock {label} has invalid SHA-256")


def validate_lock(value: object, *, expected_version: str) -> None:
    if not isinstance(value, dict):
        raise IntegrityFailure("Vanilla lock root must be an object")
    if value.get("schema_version") != LOCK_SCHEMA:
        raise IntegrityFailure(f"unsupported Vanilla lock schema {value.get('schema_version')!r}")
    minecraft = value.get("minecraft")
    if not isinstance(minecraft, dict) or minecraft.get("id") != expected_version:
        raise IntegrityFailure(f"Vanilla lock version must be {expected_version!r}")
    for name in ("metadata", "client", "server", "asset_index", "vineflower"):
        _artifact(value.get(name), name, require_sha256=name == "vineflower")
    libraries = value.get("libraries")
    if not isinstance(libraries, list) or not libraries:
        raise IntegrityFailure("Vanilla lock libraries must be a non-empty array")
    paths: list[str] = []
    for index, library in enumerate(libraries):
        _artifact(library, f"libraries[{index}]")
        paths.append(library["path"])
    if paths != sorted(paths) or len(paths) != len(set(paths)):
        raise IntegrityFailure("Vanilla lock library paths must be sorted and unique")
    java = value.get("java")
    if not isinstance(java, dict) or not isinstance(java.get("major"), int):
        raise IntegrityFailure("Vanilla lock Java major is missing")
    decompile = value.get("decompile")
    if not isinstance(decompile, dict) or not isinstance(decompile.get("arguments"), list):
        raise IntegrityFailure("Vanilla lock decompile arguments are missing")
