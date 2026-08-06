"""Explicit online discovery for reviewable lock candidates."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

from .download import fetch_bytes
from .errors import DownloadFailure, IntegrityFailure
from .lockfile import canonical_json, validate_lock

VERSION_MANIFEST_URL = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
VINEFLOWER_VERSION = "1.12.0"
VINEFLOWER_URL = (
    "https://repo1.maven.org/maven2/org/vineflower/vineflower/"
    f"{VINEFLOWER_VERSION}/vineflower-{VINEFLOWER_VERSION}.jar"
)


def _json(payload: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(payload)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise IntegrityFailure(f"cannot parse downloaded {label}: {error}") from error
    if not isinstance(value, dict):
        raise IntegrityFailure(f"downloaded {label} must be a JSON object")
    return value


def _locked_artifact(
    *,
    path: str,
    value: dict[str, Any],
    payload: bytes | None = None,
) -> dict[str, Any]:
    try:
        artifact = {
            "path": path,
            "url": value["url"],
            "size": value["size"],
            "sha1": value["sha1"],
        }
    except KeyError as error:
        raise IntegrityFailure(f"download descriptor lacks {error.args[0]}") from error
    if payload is not None:
        artifact["sha256"] = hashlib.sha256(payload).hexdigest()
    return artifact


def build_lock_candidate(version: str) -> dict[str, Any]:
    manifest_payload = fetch_bytes(VERSION_MANIFEST_URL)
    manifest = _json(manifest_payload, "version manifest")
    versions = manifest.get("versions")
    entry = next(
        (
            value
            for value in versions or []
            if isinstance(value, dict) and value.get("id") == version
        ),
        None,
    )
    if entry is None:
        raise DownloadFailure(f"Minecraft version {version!r} is absent from the manifest")
    metadata_payload = fetch_bytes(entry["url"])
    if hashlib.sha1(metadata_payload).hexdigest() != entry.get("sha1"):
        raise IntegrityFailure("downloaded version metadata disagrees with the manifest")
    metadata = _json(metadata_payload, "version metadata")
    if metadata.get("id") != version:
        raise IntegrityFailure(
            f"downloaded metadata id is {metadata.get('id')!r}, expected {version!r}"
        )
    asset = metadata.get("assetIndex")
    if not isinstance(asset, dict):
        raise IntegrityFailure("version metadata lacks assetIndex")
    asset_payload = fetch_bytes(asset["url"])
    if hashlib.sha1(asset_payload).hexdigest() != asset.get("sha1"):
        raise IntegrityFailure("downloaded asset index disagrees with metadata")
    vineflower_payload = fetch_bytes(VINEFLOWER_URL)
    libraries: list[dict[str, Any]] = []
    for library in metadata.get("libraries", []):
        if not isinstance(library, dict):
            continue
        source = library.get("downloads", {}).get("artifact")
        if not isinstance(source, dict):
            continue
        item = _locked_artifact(path=f"libraries/{source['path']}", value=source)
        item["name"] = library.get("name", source["path"])
        if "rules" in library:
            item["rules"] = library["rules"]
        libraries.append(item)
    libraries.sort(key=lambda item: item["path"])
    client = metadata.get("downloads", {}).get("client")
    server = metadata.get("downloads", {}).get("server")
    if not isinstance(client, dict) or not isinstance(server, dict):
        raise IntegrityFailure("version metadata lacks client or server download")
    java = metadata.get("javaVersion", {}).get("majorVersion")
    if not isinstance(java, int):
        raise IntegrityFailure("version metadata lacks Java major")
    candidate = {
        "schema_version": 1,
        "minecraft": {
            "id": version,
            "discovery_manifest_url": VERSION_MANIFEST_URL,
            "metadata_manifest_sha1": entry["sha1"],
        },
        "metadata": {
            "path": f"{version}.json",
            "url": entry["url"],
            "size": len(metadata_payload),
            "sha1": entry["sha1"],
            "sha256": hashlib.sha256(metadata_payload).hexdigest(),
        },
        "client": _locked_artifact(path=f"client-{version}.jar", value=client),
        "server": _locked_artifact(path=f"server-{version}.jar", value=server),
        "libraries": libraries,
        "asset_index": {
            **_locked_artifact(
                path=f"assets/indexes/{asset['id']}.json",
                value=asset,
                payload=asset_payload,
            ),
            "id": asset["id"],
            "total_size": asset["totalSize"],
        },
        "assets": {"object_base_url": "https://resources.download.minecraft.net"},
        "vineflower": {
            "path": f"tools/vineflower-{VINEFLOWER_VERSION}.jar",
            "url": VINEFLOWER_URL,
            "size": len(vineflower_payload),
            "sha1": hashlib.sha1(vineflower_payload).hexdigest(),
            "sha256": hashlib.sha256(vineflower_payload).hexdigest(),
            "version": VINEFLOWER_VERSION,
        },
        "java": {"major": java},
        "decompile": {
            "arguments": [
                "--folder",
                "--include-runtime=current",
                "--skip-extra-files=false",
                "--log-level=warn",
                "--thread-count={threads}",
            ],
            "max_threads": 32,
            "timeout_seconds": 1800,
        },
    }
    validate_lock(candidate, expected_version=version)
    return candidate


def write_candidate(version: str, output: Path) -> dict[str, Any]:
    candidate = build_lock_candidate(version)
    output = output.expanduser().absolute()
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(f".{output.name}.tmp")
    temporary.write_bytes(canonical_json(candidate))
    temporary.replace(output)
    return {
        "status": "lock-candidate-written",
        "version": version,
        "path": str(output),
        "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
    }
