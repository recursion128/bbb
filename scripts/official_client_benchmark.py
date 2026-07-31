#!/usr/bin/env python3
"""Official Minecraft 26.1 equal-load benchmark support.

This module intentionally keeps network downloads outside the benchmark path.
It validates a provisioned Mojang distribution, prepares an unmodified offline
client launch, generates the server-authoritative camera replay datapack, and
parses common external capture artifacts.
"""

from __future__ import annotations

import argparse
import csv
import grp
import hashlib
import json
import math
import os
import platform
import pwd
import re
import shutil
import signal
import socket
import stat
import subprocess
import sys
import time
import zipfile
from dataclasses import asdict, dataclass
from pathlib import Path, PurePosixPath
from typing import Any, Iterable, Mapping, Sequence


BENCHMARK_SCHEMA = 1
REPLAY_SCHEMA = 1
MINECRAFT_VERSION = "26.1"
BENCHMARK_USERNAME = "bbb-benchmark"
OFFLINE_ACCESS_TOKEN = "offline"
WINDOW_WIDTH = 1280
WINDOW_HEIGHT = 720
COMPOSITOR_WIDTH = 1600
COMPOSITOR_HEIGHT = 900
VIEW_DISTANCE = 12
SIMULATION_DISTANCE = 12
SERVER_SEED = "bbb-official-benchmark-26.1"
SERVER_READY_MARKER = "Done ("
WORLD_PREPARED_MARKER = "BBB_BENCHMARK_WORLD_PREPARED"
DEFAULT_HITCH_THRESHOLDS_MS = (33.333, 50.0, 100.0)
REQUIRED_PERF_EVENTS = (
    "task-clock",
    "cycles",
    "instructions",
    "context-switches",
    "cpu-migrations",
    "page-faults",
)
REQUIRED_FTRACE_EVENTS = (
    "amdgpu:amdgpu_cs_ioctl",
    "amdgpu:amdgpu_sched_run_job",
    "fence:fence_signaled",
    "drm:drm_vblank_event",
    "sched:sched_switch",
)
MANGOHUD_VERSION = "0.8.4"
FIXED_WARMUP_SECONDS = 30
MANGOHUD_CONTROL_PREFIX = "bbb-benchmark-mango"
MANGOHUD_RELEASE_URL = (
    "https://github.com/flightlessmango/MangoHud/releases/download/"
    "v0.8.4/MangoHud-0.8.4.r0.g992103e.tar.gz"
)
MANGOHUD_ARCHIVE_SHA256 = (
    "aa0b2a009552f5e7c50baabf361267a79ea5e5c0fea440786f3b202d51f57753"
)
MANGOHUD_FILE_SHA256 = {
    "usr/bin/mangohud": "18e1e1a710a24bd87e5306e05022ea0ff6c47536ec55450476306bdffaad3b73",
    "usr/lib/mangohud/lib64/libMangoHud.so": (
        "7aef0abfbc2aed5de0266cf5c1c6672b1e8d79c9de74ea97b3229ba975e16d11"
    ),
    "usr/lib/mangohud/lib64/libMangoHud_opengl.so": (
        "95cbda569061330be67439f6b98c3978d1ad7e53c8b52d1e6891fdc9a2f03fd4"
    ),
    "usr/lib/mangohud/lib64/libMangoHud_shim.so": (
        "46db765a49625354d3fb5735bb672a35ca39dd3a3c03796fd48ef8d17fec2280"
    ),
    "usr/share/vulkan/implicit_layer.d/MangoHud.x86_64.json": (
        "bfafa2cd3588a7c3fdb7c7c3c5df11981d0fdf032799cdf9d9f54f2b9e03c5d3"
    ),
}


class BenchmarkFailure(RuntimeError):
    """A benchmark contract failure with actionable text."""


@dataclass(frozen=True)
class VerifiedFile:
    path: str
    sha1: str
    size: int


@dataclass(frozen=True)
class DistributionAudit:
    schema: int
    version: str
    metadata: VerifiedFile
    client: VerifiedFile
    libraries: tuple[VerifiedFile, ...]
    native_classifiers: tuple[VerifiedFile, ...]
    asset_index: VerifiedFile
    asset_objects: int
    asset_object_bytes: int
    asset_index_id: str
    main_class: str
    resource_hash: str


@dataclass(frozen=True)
class LaunchPlan:
    schema: int
    client: str
    version: str
    username: str
    server: str
    game_dir: str
    assets_dir: str
    asset_index: str
    natives_dir: str
    width: int
    height: int
    argv: tuple[str, ...]


@dataclass(frozen=True)
class FrameSummary:
    samples: int
    p50_ms: float
    p95_ms: float
    p99_ms: float
    one_percent_low_fps: float
    mean_fps: float
    hitch_counts: Mapping[str, int]
    gpu_load_mean: float | None
    gpu_clock_mean_mhz: float | None
    gpu_temp_mean_c: float | None


@dataclass(frozen=True)
class WorldSnapshot:
    schema: int
    minecraft_version: str
    replay_hash: str
    server: VerifiedFile
    world: str
    world_hash: str
    server_seed: str
    preparation_commands: tuple[str, ...]


def equal_load_settings() -> dict[str, Any]:
    return {
        "schema": BENCHMARK_SCHEMA,
        "window": {"width": WINDOW_WIDTH, "height": WINDOW_HEIGHT, "mode": "windowed"},
        "view_distance": VIEW_DISTANCE,
        "simulation_distance": SIMULATION_DISTANCE,
        "entity_distance_scaling": 1.0,
        "graphics": "fancy",
        "ambient_occlusion": True,
        "biome_blend_radius": 2,
        "prioritize_chunk_updates": "player_affected",
        "clouds": True,
        "cloud_distance": 64,
        "particles": "all",
        "mipmap_levels": 4,
        "cutout_leaves": True,
        "entity_shadows": True,
        "improved_transparency": False,
        "weather_radius": 10,
        "max_anisotropy_bit": 1,
        "texture_filtering": "rgss",
        "hud_hidden": False,
        "gui_scale": 1,
        "view_bobbing": True,
        "audio": "disabled",
        "vsync": False,
        "framerate_limit": "unlimited",
        "resource_packs": ["vanilla"],
    }


def canonical_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode("utf-8")


def canonical_sha256(value: Any) -> str:
    return hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def file_digest(path: Path, algorithm: str = "sha1") -> str:
    digest = hashlib.new(algorithm)
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_file(
    path: Path,
    *,
    expected_sha1: str,
    expected_size: int | None,
    label: str,
) -> VerifiedFile:
    if not path.is_file():
        raise BenchmarkFailure(f"{label} is missing: {path}")
    size = path.stat().st_size
    if expected_size is not None and size != expected_size:
        raise BenchmarkFailure(
            f"{label} size mismatch: expected {expected_size}, got {size}: {path}"
        )
    actual_sha1 = file_digest(path)
    if actual_sha1 != expected_sha1:
        raise BenchmarkFailure(
            f"{label} SHA-1 mismatch: expected {expected_sha1}, "
            f"got {actual_sha1}: {path}"
        )
    return VerifiedFile(str(path), actual_sha1, size)


def validate_sha256(path: Path, *, expected: str, label: str) -> dict[str, Any]:
    if not path.is_file():
        raise BenchmarkFailure(f"{label} is missing: {path}")
    actual = file_digest(path, "sha256")
    if actual != expected:
        raise BenchmarkFailure(
            f"{label} SHA-256 mismatch: expected {expected}, got {actual}: {path}"
        )
    return {"path": str(path), "sha256": actual, "size": path.stat().st_size}


def load_json_object(path: Path, label: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BenchmarkFailure(f"cannot read {label} {path}: {error}") from error
    if not isinstance(value, dict):
        raise BenchmarkFailure(f"{label} must contain a JSON object: {path}")
    return value


def _rule_matches(
    rule: Mapping[str, Any],
    *,
    os_name: str,
    os_arch: str,
    os_version: str,
    features: Mapping[str, bool],
) -> bool:
    os_rule = rule.get("os", {})
    if os_rule:
        if not isinstance(os_rule, dict):
            raise BenchmarkFailure("library rule os field must be an object")
        if os_rule.get("name") not in (None, os_name):
            return False
        if os_rule.get("arch") not in (None, os_arch):
            return False
        version_pattern = os_rule.get("version")
        if version_pattern is not None:
            try:
                if re.search(str(version_pattern), os_version) is None:
                    return False
            except re.error as error:
                raise BenchmarkFailure(
                    f"invalid library OS version regex {version_pattern!r}"
                ) from error
    feature_rule = rule.get("features", {})
    if feature_rule:
        if not isinstance(feature_rule, dict):
            raise BenchmarkFailure("library rule features field must be an object")
        for name, expected in feature_rule.items():
            if features.get(str(name), False) is not bool(expected):
                return False
    return True


def library_allowed(
    library: Mapping[str, Any],
    *,
    os_name: str = "linux",
    os_arch: str = "x86_64",
    os_version: str = "",
    features: Mapping[str, bool] | None = None,
) -> bool:
    rules = library.get("rules")
    if not rules:
        return True
    if not isinstance(rules, list):
        raise BenchmarkFailure("library rules must be a list")
    allowed = False
    for rule in rules:
        if not isinstance(rule, dict):
            raise BenchmarkFailure("library rule must be an object")
        if _rule_matches(
            rule,
            os_name=os_name,
            os_arch=os_arch,
            os_version=os_version,
            features=features or {},
        ):
            action = rule.get("action")
            if action not in ("allow", "disallow"):
                raise BenchmarkFailure(f"unknown library rule action: {action!r}")
            allowed = action == "allow"
    return allowed


def _safe_relative_artifact_path(raw: Any, label: str) -> Path:
    if not isinstance(raw, str) or not raw:
        raise BenchmarkFailure(f"{label} path is missing")
    posix = PurePosixPath(raw)
    if posix.is_absolute() or ".." in posix.parts or "\\" in raw:
        raise BenchmarkFailure(f"{label} path escapes the library root: {raw!r}")
    return Path(*posix.parts)


def _native_classifier(name: str) -> bool:
    parts = name.split(":")
    return len(parts) >= 4 and parts[3].startswith("natives-")


def audit_distribution(
    mc_root: Path,
    *,
    java_path: Path,
    verify_asset_objects: bool = True,
) -> DistributionAudit:
    downloads = mc_root / "downloads" / MINECRAFT_VERSION
    manifest_path = downloads / "version_manifest_v2.json"
    metadata_path = downloads / f"{MINECRAFT_VERSION}.json"
    libraries_root = downloads / "libraries"
    assets_root = downloads / "assets"
    client_path = downloads / f"client-{MINECRAFT_VERSION}.jar"

    if not java_path.is_file() or not os.access(java_path, os.X_OK):
        raise BenchmarkFailure(f"Java executable is unavailable: {java_path}")

    manifest = load_json_object(manifest_path, "version manifest")
    versions = manifest.get("versions")
    if not isinstance(versions, list):
        raise BenchmarkFailure("version manifest lacks versions")
    manifest_entry = next(
        (
            entry
            for entry in versions
            if isinstance(entry, dict) and entry.get("id") == MINECRAFT_VERSION
        ),
        None,
    )
    if manifest_entry is None or not isinstance(manifest_entry.get("sha1"), str):
        raise BenchmarkFailure(f"version manifest lacks {MINECRAFT_VERSION}")
    metadata_verified = validate_file(
        metadata_path,
        expected_sha1=manifest_entry["sha1"],
        expected_size=None,
        label="version metadata",
    )
    metadata = load_json_object(metadata_path, "version metadata")
    if metadata.get("id") != MINECRAFT_VERSION:
        raise BenchmarkFailure(
            f"version metadata id must be {MINECRAFT_VERSION!r}, "
            f"got {metadata.get('id')!r}"
        )
    main_class = metadata.get("mainClass")
    if not isinstance(main_class, str) or not main_class:
        raise BenchmarkFailure("version metadata lacks mainClass")

    client_download = metadata.get("downloads", {}).get("client")
    if not isinstance(client_download, dict):
        raise BenchmarkFailure("version metadata lacks client download")
    client_verified = validate_file(
        client_path,
        expected_sha1=str(client_download.get("sha1", "")),
        expected_size=int(client_download.get("size", -1)),
        label="official client jar",
    )

    libraries: list[VerifiedFile] = []
    natives: list[VerifiedFile] = []
    raw_libraries = metadata.get("libraries")
    if not isinstance(raw_libraries, list):
        raise BenchmarkFailure("version metadata lacks libraries")
    for library in raw_libraries:
        if not isinstance(library, dict) or not library_allowed(
            library,
            os_name="linux",
            os_arch=platform.machine(),
            os_version=platform.release(),
        ):
            continue
        name = library.get("name")
        artifact = library.get("downloads", {}).get("artifact")
        if not isinstance(name, str) or not isinstance(artifact, dict):
            raise BenchmarkFailure("allowed library lacks name/download artifact")
        relative = _safe_relative_artifact_path(
            artifact.get("path"), f"library {name}"
        )
        verified = validate_file(
            libraries_root / relative,
            expected_sha1=str(artifact.get("sha1", "")),
            expected_size=int(artifact.get("size", -1)),
            label=f"library {name}",
        )
        (natives if _native_classifier(name) else libraries).append(verified)

    asset_index = metadata.get("assetIndex")
    if not isinstance(asset_index, dict):
        raise BenchmarkFailure("version metadata lacks assetIndex")
    asset_index_id = str(asset_index.get("id", ""))
    asset_index_path = assets_root / "indexes" / f"{asset_index_id}.json"
    asset_index_verified = validate_file(
        asset_index_path,
        expected_sha1=str(asset_index.get("sha1", "")),
        expected_size=int(asset_index.get("size", -1)),
        label="asset index",
    )
    asset_index_json = load_json_object(asset_index_path, "asset index")
    objects = asset_index_json.get("objects")
    if not isinstance(objects, dict):
        raise BenchmarkFailure("asset index lacks objects")
    total_object_bytes = 0
    resource_entries: list[tuple[str, str, int]] = []
    for logical_name, info in sorted(objects.items()):
        if not isinstance(logical_name, str) or not isinstance(info, dict):
            raise BenchmarkFailure("asset index contains malformed object")
        digest = info.get("hash")
        size = info.get("size")
        if (
            not isinstance(digest, str)
            or re.fullmatch(r"[0-9a-f]{40}", digest) is None
            or not isinstance(size, int)
            or size < 0
        ):
            raise BenchmarkFailure(f"invalid asset object metadata: {logical_name}")
        object_path = assets_root / "objects" / digest[:2] / digest
        if verify_asset_objects:
            validate_file(
                object_path,
                expected_sha1=digest,
                expected_size=size,
                label=f"asset object {logical_name}",
            )
        total_object_bytes += size
        resource_entries.append((logical_name, digest, size))
    if total_object_bytes != int(asset_index.get("totalSize", -1)):
        raise BenchmarkFailure(
            "asset index totalSize mismatch: "
            f"metadata={asset_index.get('totalSize')}, index={total_object_bytes}"
        )

    resource_hash = canonical_sha256(
        {
            "version_metadata": metadata_verified.sha1,
            "client": client_verified.sha1,
            "libraries": [entry.sha1 for entry in libraries],
            "natives": [entry.sha1 for entry in natives],
            "asset_index": asset_index_verified.sha1,
            "objects": resource_entries,
        }
    )
    return DistributionAudit(
        schema=BENCHMARK_SCHEMA,
        version=MINECRAFT_VERSION,
        metadata=metadata_verified,
        client=client_verified,
        libraries=tuple(libraries),
        native_classifiers=tuple(natives),
        asset_index=asset_index_verified,
        asset_objects=len(objects),
        asset_object_bytes=total_object_bytes,
        asset_index_id=asset_index_id,
        main_class=main_class,
        resource_hash=resource_hash,
    )


def validate_server_jar(mc_root: Path) -> VerifiedFile:
    downloads = mc_root / "downloads" / MINECRAFT_VERSION
    metadata = load_json_object(
        downloads / f"{MINECRAFT_VERSION}.json", "version metadata"
    )
    server_download = metadata.get("downloads", {}).get("server")
    if not isinstance(server_download, dict):
        raise BenchmarkFailure("version metadata lacks server download")
    return validate_file(
        downloads / f"server-{MINECRAFT_VERSION}.jar",
        expected_sha1=str(server_download.get("sha1", "")),
        expected_size=int(server_download.get("size", -1)),
        label="official server jar",
    )


def _validate_zip_member(info: zipfile.ZipInfo, archive: Path) -> PurePosixPath:
    raw = info.filename
    if "\x00" in raw or "\\" in raw:
        raise BenchmarkFailure(f"unsafe native archive path in {archive}: {raw!r}")
    path = PurePosixPath(raw)
    if path.is_absolute() or ".." in path.parts:
        raise BenchmarkFailure(f"native archive path escapes target: {raw!r}")
    mode = info.external_attr >> 16
    if stat.S_ISLNK(mode):
        raise BenchmarkFailure(f"native archive contains symlink: {raw!r}")
    return path


def safe_extract_natives(
    native_archives: Iterable[Path], destination: Path
) -> tuple[VerifiedFile, ...]:
    destination.mkdir(parents=True, exist_ok=True)
    extracted: dict[str, VerifiedFile] = {}
    for archive in native_archives:
        try:
            jar = zipfile.ZipFile(archive)
        except (OSError, zipfile.BadZipFile) as error:
            raise BenchmarkFailure(f"cannot open native archive {archive}: {error}") from error
        with jar:
            for info in jar.infolist():
                member = _validate_zip_member(info, archive)
                if info.is_dir() or not member.name.endswith(".so"):
                    continue
                payload = jar.read(info)
                digest = hashlib.sha1(payload).hexdigest()
                output = destination / member.name
                existing = extracted.get(member.name)
                if existing is not None:
                    if existing.sha1 != digest or existing.size != len(payload):
                        raise BenchmarkFailure(
                            f"native basename collision for {member.name}"
                        )
                    continue
                if output.exists():
                    if (
                        not output.is_file()
                        or output.stat().st_size != len(payload)
                        or file_digest(output) != digest
                    ):
                        raise BenchmarkFailure(
                            f"refusing to replace native extraction target: {output}"
                        )
                else:
                    temporary = output.with_name(output.name + ".part")
                    temporary.write_bytes(payload)
                    os.replace(temporary, output)
                    output.chmod(0o755)
                verified = VerifiedFile(str(output), digest, len(payload))
                extracted[member.name] = verified
    if not extracted:
        raise BenchmarkFailure("Linux native classifiers contained no .so files")
    return tuple(extracted[name] for name in sorted(extracted))


def vanilla_options() -> dict[str, str]:
    return {
        # SharedConstants.WORLD_VERSION in the 26.1 source baseline.
        "version": "4786",
        "fullscreen": "false",
        "enableVsync": "false",
        "maxFps": "260",
        "renderDistance": str(VIEW_DISTANCE),
        "simulationDistance": str(SIMULATION_DISTANCE),
        "entityDistanceScaling": "1.0",
        # Minecraft applies a named preset after loading every option. CUSTOM
        # preserves the explicit Fancy-equivalent component values below while
        # allowing the shared 12-chunk distance and zero menu blur overrides.
        "graphicsPreset": '"custom"',
        "ao": "true",
        "biomeBlendRadius": "2",
        "prioritizeChunkUpdates": "1",
        "renderClouds": '"true"',
        "cloudRange": "64",
        "particles": "0",
        "mipmapLevels": "4",
        "cutoutLeaves": "true",
        "entityShadows": "true",
        "improvedTransparency": "false",
        "weatherRadius": "10",
        "maxAnisotropyBit": "1",
        "textureFiltering": "1",
        "guiScale": "1",
        "bobView": "true",
        "soundCategory_master": "0.0",
        "lang": "en_us",
        "tutorialStep": "none",
        "joinedFirstServer": "true",
        "onboardAccessibility": "false",
        "menuBackgroundBlurriness": "0",
        "resourcePacks": '["vanilla"]',
        "incompatibleResourcePacks": "[]",
    }


def write_vanilla_options(game_dir: Path) -> Path:
    game_dir.mkdir(parents=True, exist_ok=True)
    options_path = game_dir / "options.txt"
    payload = "".join(f"{key}:{value}\n" for key, value in vanilla_options().items())
    options_path.write_text(payload, encoding="utf-8")
    return options_path


def audit_vanilla_options(path: Path) -> dict[str, Any]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise BenchmarkFailure(f"cannot read Vanilla options {path}: {error}") from error
    values: dict[str, str] = {}
    for line in lines:
        if not line:
            continue
        key, separator, value = line.partition(":")
        if not separator or not key:
            raise BenchmarkFailure(f"malformed Vanilla option line: {line!r}")
        if key in values:
            raise BenchmarkFailure(f"duplicate Vanilla option key: {key}")
        values[key] = value

    mismatches: list[str] = []
    audited: dict[str, Any] = {}
    for key, expected_raw in vanilla_options().items():
        actual_raw = values.get(key)
        if actual_raw is None:
            mismatches.append(f"{key}=<missing>")
            continue
        try:
            expected_value = json.loads(expected_raw)
        except json.JSONDecodeError:
            expected_value = expected_raw
        try:
            actual_value = json.loads(actual_raw)
        except json.JSONDecodeError:
            actual_value = actual_raw
        audited[key] = actual_value
        if actual_value != expected_value:
            mismatches.append(
                f"{key}: expected {expected_value!r}, got {actual_value!r}"
            )
    if mismatches:
        raise BenchmarkFailure(
            "Vanilla loaded options differ from the equal-load contract: "
            + "; ".join(mismatches)
        )
    return {
        "path": str(path),
        "sha256": file_digest(path, "sha256"),
        "values": audited,
    }


def build_vanilla_launch_plan(
    audit: DistributionAudit,
    *,
    java_path: Path,
    game_dir: Path,
    assets_dir: Path,
    natives_dir: Path,
    server_port: int,
) -> LaunchPlan:
    if not 1 <= server_port <= 65535:
        raise BenchmarkFailure(f"invalid server port: {server_port}")
    classpath = os.pathsep.join(
        [entry.path for entry in audit.libraries] + [audit.client.path]
    )
    argv = (
        str(java_path),
        "-Xms2G",
        "-Xmx4G",
        "-XX:+UseCompactObjectHeaders",
        "-XX:+AlwaysPreTouch",
        "-XX:+UseStringDeduplication",
        "-XX:+UseZGC",
        "-DMC_DEBUG_ENABLED",
        "-DMC_DEBUG_PREFER_WAYLAND",
        f"-Djava.library.path={natives_dir}",
        "-cp",
        classpath,
        audit.main_class,
        "--offlineDeveloperMode",
        "--username",
        BENCHMARK_USERNAME,
        "--accessToken",
        OFFLINE_ACCESS_TOKEN,
        "--version",
        MINECRAFT_VERSION,
        "--gameDir",
        str(game_dir),
        "--assetsDir",
        str(assets_dir),
        "--assetIndex",
        audit.asset_index_id,
        "--width",
        str(WINDOW_WIDTH),
        "--height",
        str(WINDOW_HEIGHT),
        "--quickPlayMultiplayer",
        f"127.0.0.1:{server_port}",
    )
    return LaunchPlan(
        schema=BENCHMARK_SCHEMA,
        client="vanilla",
        version=MINECRAFT_VERSION,
        username=BENCHMARK_USERNAME,
        server=f"127.0.0.1:{server_port}",
        game_dir=str(game_dir),
        assets_dir=str(assets_dir),
        asset_index=audit.asset_index_id,
        natives_dir=str(natives_dir),
        width=WINDOW_WIDTH,
        height=WINDOW_HEIGHT,
        argv=argv,
    )


def build_bbb_launch_argv(
    *,
    binary: Path,
    server_port: int,
    control_port: int,
) -> tuple[str, ...]:
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise BenchmarkFailure(f"bbb-native executable is unavailable: {binary}")
    return (
        str(binary),
        "--server",
        f"127.0.0.1:{server_port}",
        "--username",
        BENCHMARK_USERNAME,
        "--connect-server",
        "--control",
        f"127.0.0.1:{control_port}",
        "--client-view-distance",
        str(VIEW_DISTANCE),
        "--render-distance",
        str(VIEW_DISTANCE),
        "--client-particles",
        "all",
        "--tutorial-step",
        "none",
        "--client-vsync",
        "false",
        "--client-framerate-limit",
        "260",
        "--cutout-leaves",
        "true",
        "--entity-shadows",
        "true",
        "--menu-background-blurriness",
        "0",
    )


def sanitized_argv(argv: Sequence[str]) -> list[str]:
    sanitized = list(argv)
    for index, value in enumerate(sanitized[:-1]):
        if value == "--accessToken":
            sanitized[index + 1] = "<offline-placeholder>"
    return sanitized


def validate_replay(replay: Mapping[str, Any]) -> None:
    if replay.get("schema") != REPLAY_SCHEMA:
        raise BenchmarkFailure(f"replay schema must be {REPLAY_SCHEMA}")
    if replay.get("minecraft_version") != MINECRAFT_VERSION:
        raise BenchmarkFailure(f"replay version must be {MINECRAFT_VERSION}")
    if replay.get("player") != BENCHMARK_USERNAME:
        raise BenchmarkFailure(f"replay player must be {BENCHMARK_USERNAME}")
    frames = replay.get("frames")
    if not isinstance(frames, list) or not frames:
        raise BenchmarkFailure("replay frames must be a non-empty list")
    expected_tick = 0
    segments: dict[str, set[str]] = {}
    for frame in frames:
        if not isinstance(frame, dict):
            raise BenchmarkFailure("replay frame must be an object")
        tick = frame.get("server_tick")
        if tick != expected_tick:
            raise BenchmarkFailure(
                f"replay server_tick must be contiguous: expected {expected_tick}, "
                f"got {tick!r}"
            )
        expected_tick += 1
        for name in ("x", "y", "z", "yaw", "pitch"):
            value = frame.get(name)
            if (
                not isinstance(value, (int, float))
                or isinstance(value, bool)
                or not math.isfinite(value)
            ):
                raise BenchmarkFailure(f"replay frame {tick} has invalid {name}")
        if not -180.0 <= float(frame["yaw"]) <= 180.0:
            raise BenchmarkFailure(f"replay frame {tick} yaw is out of range")
        if not -90.0 <= float(frame["pitch"]) <= 90.0:
            raise BenchmarkFailure(f"replay frame {tick} pitch is out of range")
        segment = frame.get("capture_segment")
        marker = frame.get("marker")
        if not isinstance(segment, str) or not segment:
            raise BenchmarkFailure(f"replay frame {tick} lacks capture_segment")
        if marker is not None and not isinstance(marker, str):
            raise BenchmarkFailure(f"replay frame {tick} marker must be a string")
        segments.setdefault(segment, set())
        if marker:
            segments[segment].add(marker)
    expected_segments = {
        "static_orbit",
        "preloaded_flight",
        "chunk_boundary_crossing",
    }
    if set(segments) != expected_segments:
        raise BenchmarkFailure(
            f"replay segments must be {sorted(expected_segments)}, "
            f"got {sorted(segments)}"
        )
    for segment, markers in segments.items():
        for required in ("segment_start", "capture_start", "anchor", "capture_end"):
            if required not in markers:
                raise BenchmarkFailure(
                    f"replay segment {segment} lacks marker {required}"
                )


def default_replay() -> dict[str, Any]:
    frames: list[dict[str, Any]] = []
    segment_ticks = 480
    marker_ticks = {
        0: "segment_start",
        120: "capture_start",
        240: "anchor",
        479: "capture_end",
    }

    def append_segment(
        segment: str, positions: Sequence[tuple[float, float, float, float, float]]
    ) -> None:
        if len(positions) != segment_ticks:
            raise BenchmarkFailure(
                f"default replay segment {segment} must contain {segment_ticks} frames"
            )
        for offset, (x, y, z, yaw, pitch) in enumerate(positions):
            frames.append(
                {
                    "server_tick": len(frames),
                    "x": round(x, 6),
                    "y": round(y, 6),
                    "z": round(z, 6),
                    "yaw": round(yaw, 6),
                    "pitch": round(pitch, 6),
                    "capture_segment": segment,
                    "marker": marker_ticks.get(offset),
                }
            )

    orbit: list[tuple[float, float, float, float, float]] = []
    anchor_angle = math.radians(48.0)
    for offset in range(segment_ticks):
        angle = anchor_angle if 240 <= offset < 280 else math.radians(offset * 0.2)
        x = math.cos(angle) * 24.0
        z = math.sin(angle) * 24.0
        yaw = math.degrees(math.atan2(-x, z))
        orbit.append((x, 82.0, z, yaw, 12.0))
    append_segment("static_orbit", orbit)

    flight: list[tuple[float, float, float, float, float]] = []
    anchor_progress = 240.0 / (segment_ticks - 1)
    for offset in range(segment_ticks):
        progress = (
            anchor_progress if 240 <= offset < 280 else offset / (segment_ticks - 1)
        )
        x = -96.0 + 192.0 * progress
        z = 40.0 + math.sin(progress * math.tau * 2.0) * 20.0
        flight.append((x, 96.0, z, -90.0, 18.0))
    append_segment("preloaded_flight", flight)

    boundary_positions = (
        -8.5,
        7.5,
        15.5,
        16.5,
        31.5,
        32.5,
        47.5,
        48.5,
        63.5,
        64.5,
        79.5,
        80.5,
        95.5,
        96.5,
        111.5,
        112.5,
        127.5,
        128.5,
        143.5,
        144.5,
    )
    crossing: list[tuple[float, float, float, float, float]] = []
    hold_ticks = segment_ticks // len(boundary_positions)
    for offset in range(segment_ticks):
        if 240 <= offset < 280:
            x = 79.5
        else:
            index = min(offset // hold_ticks, len(boundary_positions) - 1)
            x = boundary_positions[index]
        crossing.append((x, 88.0, -48.5, -90.0, 20.0))
    append_segment("chunk_boundary_crossing", crossing)

    replay = {
        "schema": REPLAY_SCHEMA,
        "minecraft_version": MINECRAFT_VERSION,
        "player": BENCHMARK_USERNAME,
        "tick_rate": 20,
        "coordinate_contract": "absolute server tp; yaw then pitch",
        "segments": [
            {
                "id": "static_orbit",
                "purpose": "stable draw and pipeline load with a slow orbit",
                "ticks": segment_ticks,
            },
            {
                "id": "preloaded_flight",
                "purpose": "frustum, entity extraction, and translucent ordering",
                "ticks": segment_ticks,
            },
            {
                "id": "chunk_boundary_crossing",
                "purpose": "step-and-hold chunk update, meshing, and upload tails",
                "ticks": segment_ticks,
            },
        ],
        "frames": frames,
    }
    validate_replay(replay)
    return replay


def replay_hash(replay: Mapping[str, Any]) -> str:
    validate_replay(replay)
    return canonical_sha256(replay)


def _format_command_number(value: float) -> str:
    formatted = f"{value:.6f}".rstrip("0").rstrip(".")
    return "0" if formatted in ("-0", "") else formatted


def generate_replay_datapack(replay: Mapping[str, Any], output: Path) -> str:
    digest = replay_hash(replay)
    function_dir = output / "data" / "bbb_benchmark" / "function"
    tag_dir = output / "data" / "minecraft" / "tags" / "function"
    function_dir.mkdir(parents=True, exist_ok=True)
    tag_dir.mkdir(parents=True, exist_ok=True)
    (output / "pack.mcmeta").write_bytes(
        canonical_json_bytes(
            {
                "pack": {
                    "description": "bbb official client benchmark replay 26.1",
                    "min_format": [101, 1],
                    "max_format": [101, 1],
                }
            }
        )
    )
    (tag_dir / "load.json").write_bytes(
        canonical_json_bytes({"values": ["bbb_benchmark:load"]})
    )
    (tag_dir / "tick.json").write_bytes(
        canonical_json_bytes({"values": ["bbb_benchmark:tick"]})
    )
    (function_dir / "load.mcfunction").write_text(
        "scoreboard objectives add bbb_replay dummy\n"
        "scoreboard objectives add bbb_replay_control dummy\n"
        "scoreboard players set #run bbb_replay_control 0\n"
        f"say BBB_BENCHMARK_REPLAY {digest}\n",
        encoding="utf-8",
    )
    final_tick = len(replay["frames"]) - 1
    tick_commands = [
        (
            "execute if score #run bbb_replay_control matches 1 "
            f"as @a[name={BENCHMARK_USERNAME}] "
            "unless score @s bbb_replay matches 0.. run "
            "scoreboard players set @s bbb_replay 0"
        ),
        (
            "execute if score #run bbb_replay_control matches 1 "
            f"as @a[name={BENCHMARK_USERNAME},"
            "scores={bbb_replay=0}] run gamemode spectator @s"
        ),
        (
            "execute if score #run bbb_replay_control matches 1 "
            f"as @a[name={BENCHMARK_USERNAME},"
            f"scores={{bbb_replay=0..{final_tick}}}] "
            f"run function bbb_benchmark:dispatch/0_{final_tick}"
        ),
    ]
    (function_dir / "tick.mcfunction").write_text(
        "\n".join(tick_commands) + "\n", encoding="utf-8"
    )
    dispatch_dir = function_dir / "dispatch"
    if dispatch_dir.exists():
        shutil.rmtree(dispatch_dir)
    dispatch_dir.mkdir()
    frames = replay["frames"]

    def write_dispatch(first: int, last: int) -> None:
        path = dispatch_dir / f"{first}_{last}.mcfunction"
        if last - first + 1 > 16:
            middle = (first + last) // 2
            path.write_text(
                (
                    f"execute if score @s bbb_replay matches "
                    f"{first}..{middle} run function "
                    f"bbb_benchmark:dispatch/{first}_{middle}\n"
                    f"execute if score @s bbb_replay matches "
                    f"{middle + 1}..{last} run function "
                    f"bbb_benchmark:dispatch/{middle + 1}_{last}\n"
                ),
                encoding="utf-8",
            )
            write_dispatch(first, middle)
            write_dispatch(middle + 1, last)
            return

        commands: list[str] = []
        for index in range(first, last + 1):
            frame = frames[index]
            tick = frame["server_tick"]
            marker = frame.get("marker")
            if marker:
                pose = " ".join(
                    _format_command_number(float(frame[name]))
                    for name in ("x", "y", "z", "yaw", "pitch")
                )
                commands.append(
                    f"execute if score @s bbb_replay matches {tick} run say "
                    f"BBB_BENCHMARK_MARKER {digest} "
                    f"{frame['capture_segment']} {marker} {tick} {pose}"
                )
            coordinates = " ".join(
                _format_command_number(float(frame[name]))
                for name in ("x", "y", "z")
            )
            yaw = _format_command_number(float(frame["yaw"]))
            pitch = _format_command_number(float(frame["pitch"]))
            commands.append(
                f"execute if score @s bbb_replay matches {tick} "
                f"run tp @s {coordinates} {yaw} {pitch}"
            )
        commands.append("scoreboard players add @s bbb_replay 1")
        path.write_text("\n".join(commands) + "\n", encoding="utf-8")

    write_dispatch(0, final_tick)
    return digest


def reserve_loopback_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def server_properties(server_port: int) -> tuple[tuple[str, str], ...]:
    if not 1 <= server_port <= 65535:
        raise BenchmarkFailure(f"invalid server port: {server_port}")
    return (
        ("online-mode", "false"),
        ("enforce-secure-profile", "false"),
        ("server-ip", "127.0.0.1"),
        ("server-port", str(server_port)),
        ("level-name", "world"),
        ("level-seed", SERVER_SEED),
        ("gamemode", "spectator"),
        ("force-gamemode", "true"),
        ("difficulty", "peaceful"),
        ("spawn-protection", "0"),
        ("view-distance", str(VIEW_DISTANCE)),
        ("simulation-distance", str(SIMULATION_DISTANCE)),
        ("entity-broadcast-range-percentage", "100"),
        ("max-players", "1"),
        ("network-compression-threshold", "256"),
        ("sync-chunk-writes", "true"),
        ("allow-flight", "true"),
        ("motd", "bbb official equal-load benchmark 26.1"),
    )


def write_server_layout(
    server_dir: Path,
    *,
    replay: Mapping[str, Any],
    server_port: int,
    accept_eula: bool,
) -> str:
    if not accept_eula:
        raise BenchmarkFailure(
            "official server startup requires explicit --accept-eula"
        )
    server_dir.mkdir(parents=True, exist_ok=True)
    properties = "".join(
        f"{name}={value}\n" for name, value in server_properties(server_port)
    )
    (server_dir / "server.properties").write_text(properties, encoding="utf-8")
    (server_dir / "eula.txt").write_text("eula=true\n", encoding="utf-8")
    return generate_replay_datapack(
        replay,
        server_dir / "world" / "datapacks" / "bbb_benchmark",
    )


def _replay_chunk_bounds(replay: Mapping[str, Any]) -> tuple[int, int, int, int]:
    validate_replay(replay)
    frames = replay["frames"]
    min_x = math.floor(min(float(frame["x"]) for frame in frames) / 16.0)
    max_x = math.floor(max(float(frame["x"]) for frame in frames) / 16.0)
    min_z = math.floor(min(float(frame["z"]) for frame in frames) / 16.0)
    max_z = math.floor(max(float(frame["z"]) for frame in frames) / 16.0)
    return (
        min_x - VIEW_DISTANCE,
        max_x + VIEW_DISTANCE,
        min_z - VIEW_DISTANCE,
        max_z + VIEW_DISTANCE,
    )


def _forceload_tiles(replay: Mapping[str, Any]) -> tuple[str, ...]:
    min_x, max_x, min_z, max_z = _replay_chunk_bounds(replay)
    commands: list[str] = []
    for chunk_x in range(min_x, max_x + 1, 16):
        tile_max_x = min(chunk_x + 15, max_x)
        for chunk_z in range(min_z, max_z + 1, 16):
            tile_max_z = min(chunk_z + 15, max_z)
            commands.append(
                "execute in minecraft:overworld run forceload add "
                f"{chunk_x * 16} {chunk_z * 16} "
                f"{tile_max_x * 16 + 15} {tile_max_z * 16 + 15}"
            )
    return tuple(commands)


def world_preparation_commands(
    replay: Mapping[str, Any],
) -> tuple[str, ...]:
    commands = [
        "gamerule advance_time false",
        "gamerule advance_weather false",
        "gamerule random_tick_speed 0",
        "gamerule spawn_mobs false",
        "gamerule spawn_monsters false",
        "gamerule spawn_patrols false",
        "gamerule spawn_wandering_traders false",
        "gamerule spawn_phantoms false",
        "gamerule fire_spread_radius_around_player -1",
        "time set noon",
        "weather clear 1000000",
        "difficulty peaceful",
        *_forceload_tiles(replay),
        "fill -16 63 -16 16 63 16 minecraft:polished_andesite",
        "fill -8 64 -8 8 72 8 minecraft:glass hollow",
        "fill -7 64 -7 7 64 7 minecraft:water",
    ]
    for index, (x, z) in enumerate(
        (
            (-6, -6),
            (-2, -6),
            (2, -6),
            (6, -6),
            (-6, -2),
            (-2, -2),
            (2, -2),
            (6, -2),
            (-6, 2),
            (-2, 2),
            (2, 2),
            (6, 2),
            (-6, 6),
            (-2, 6),
            (2, 6),
            (6, 6),
        )
    ):
        yaw = (index * 22.5) % 360.0
        commands.append(
            "summon minecraft:armor_stand "
            f"{x} 65 {z} "
            "{Tags:[\"bbb_benchmark_scene\",\"bbb_benchmark_armor\"],NoGravity:1b,"
            "Invulnerable:1b,PersistenceRequired:1b,"
            f"Rotation:[{yaw:.1f}f,0.0f]}}"
        )
    commands.extend(
        (
            "summon minecraft:cat -4 65 0 "
            "{Tags:[\"bbb_benchmark_scene\"],NoAI:1b,"
            "PersistenceRequired:1b,variant:\"minecraft:black\"}",
            "summon minecraft:cat 0 65 4 "
            "{Tags:[\"bbb_benchmark_scene\"],NoAI:1b,"
            "PersistenceRequired:1b,variant:\"minecraft:siamese\"}",
            "summon minecraft:cat 4 65 0 "
            "{Tags:[\"bbb_benchmark_scene\"],NoAI:1b,"
            "PersistenceRequired:1b,variant:\"minecraft:red\"}",
            "summon minecraft:cat 0 65 -4 "
            "{Tags:[\"bbb_benchmark_scene\"],NoAI:1b,"
            "PersistenceRequired:1b,variant:\"minecraft:calico\"}",
            "item replace entity @e[tag=bbb_benchmark_armor] "
            "armor.head with minecraft:diamond_helmet",
            "item replace entity @e[tag=bbb_benchmark_armor] "
            "armor.chest with minecraft:golden_chestplate",
            "save-all flush",
            "forceload remove all",
            "save-all flush",
            f"say {WORLD_PREPARED_MARKER} {replay_hash(replay)}",
        )
    )
    return tuple(commands)


def _reject_server_command_failures(path: Path) -> None:
    text = path.read_text(encoding="utf-8", errors="replace")
    failures = (
        "Incorrect argument for command",
        "Unknown or incomplete command",
        "<--[HERE]",
        "That position is not loaded",
        "No entity was found",
    )
    observed = [failure for failure in failures if failure in text]
    if observed:
        raise BenchmarkFailure(
            "official server rejected world preparation commands "
            f"({', '.join(observed)}); see {path}"
        )


def _wait_for_log(
    path: Path,
    marker: str,
    *,
    process: subprocess.Popen[Any],
    timeout: float,
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except FileNotFoundError:
            text = ""
        if marker in text:
            return
        status = process.poll()
        if status is not None:
            raise BenchmarkFailure(
                f"process exited with status {status} before log marker {marker!r}; "
                f"see {path}"
            )
        time.sleep(0.25)
    raise BenchmarkFailure(f"timed out waiting for log marker {marker!r}: {path}")


def _wait_for_socket(
    path: Path,
    *,
    process: subprocess.Popen[Any],
    timeout: float,
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.is_socket():
            return
        status = process.poll()
        if status is not None:
            raise BenchmarkFailure(
                f"compositor exited with status {status} before creating {path}"
            )
        time.sleep(0.1)
    raise BenchmarkFailure(f"timed out waiting for compositor socket: {path}")


def _write_server_commands(
    process: subprocess.Popen[Any], commands: Iterable[str]
) -> None:
    if process.stdin is None:
        raise BenchmarkFailure("server process has no command channel")
    for command in commands:
        process.stdin.write(command + "\n")
    process.stdin.flush()


def _warmup_and_start_replay(
    process: subprocess.Popen[Any], replay: Mapping[str, Any]
) -> None:
    first = replay["frames"][0]
    pose = " ".join(
        _format_command_number(float(first[key]))
        for key in ("x", "y", "z", "yaw", "pitch")
    )
    _write_server_commands(
        process,
        (
            "scoreboard players set #run bbb_replay_control 0",
            f"gamemode spectator {BENCHMARK_USERNAME}",
            f"tp {BENCHMARK_USERNAME} {pose}",
            "time set noon",
            "weather clear 1000000",
        ),
    )
    time.sleep(FIXED_WARMUP_SECONDS)
    _write_server_commands(
        process,
        (
            f"scoreboard players set {BENCHMARK_USERNAME} bbb_replay 0",
            "scoreboard players set #run bbb_replay_control 1",
        ),
    )


def prepare_world_snapshot(
    *,
    replay: Mapping[str, Any],
    mc_root: Path,
    java_path: Path,
    artifact_dir: Path,
    server_port: int,
    accept_eula: bool,
    timeout: float,
) -> WorldSnapshot:
    server = validate_server_jar(mc_root)
    server_dir = artifact_dir / "server"
    digest = write_server_layout(
        server_dir,
        replay=replay,
        server_port=server_port,
        accept_eula=accept_eula,
    )
    log_path = artifact_dir / "server.log"
    commands = world_preparation_commands(replay)
    with log_path.open("w", encoding="utf-8") as log:
        group = ProcessGroup(
            (
                str(java_path),
                "-Xms1G",
                "-Xmx2G",
                "-jar",
                server.path,
                "nogui",
            ),
            cwd=server_dir,
            stdin=subprocess.PIPE,
            stdout=log,
            stderr=subprocess.STDOUT,
            text=True,
        )
        try:
            process = group.start()
            _wait_for_log(
                log_path,
                SERVER_READY_MARKER,
                process=process,
                timeout=timeout,
            )
            _write_server_commands(process, commands)
            _wait_for_log(
                log_path,
                f"{WORLD_PREPARED_MARKER} {digest}",
                process=process,
                timeout=timeout,
            )
            _reject_server_command_failures(log_path)
            _write_server_commands(process, ("stop",))
            try:
                process.wait(timeout=30.0)
            except subprocess.TimeoutExpired as error:
                raise BenchmarkFailure(
                    "official server did not stop after world preparation"
                ) from error
            if process.returncode != 0:
                raise BenchmarkFailure(
                    "official server failed during world preparation; "
                    f"see {log_path}"
                )
        finally:
            group.cleanup()
    world_dir = server_dir / "world"
    snapshot = WorldSnapshot(
        schema=BENCHMARK_SCHEMA,
        minecraft_version=MINECRAFT_VERSION,
        replay_hash=digest,
        server=server,
        world=str(world_dir),
        world_hash=hash_tree(world_dir, ignored_names=("session.lock",)),
        server_seed=SERVER_SEED,
        preparation_commands=commands,
    )
    (artifact_dir / "world-snapshot.json").write_bytes(
        canonical_json_bytes(asdict(snapshot))
    )
    return snapshot


def hash_tree(root: Path, *, ignored_names: Iterable[str] = ()) -> str:
    if not root.is_dir():
        raise BenchmarkFailure(f"tree root is missing: {root}")
    ignored = set(ignored_names)
    entries: list[dict[str, Any]] = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        if any(part in ignored for part in path.relative_to(root).parts):
            continue
        if path.is_symlink():
            entries.append({"path": relative, "symlink": os.readlink(path)})
        elif path.is_file():
            entries.append(
                {
                    "path": relative,
                    "size": path.stat().st_size,
                    "sha256": file_digest(path, "sha256"),
                }
            )
        elif path.is_dir():
            entries.append({"path": relative, "directory": True})
        else:
            raise BenchmarkFailure(f"unsupported tree entry: {path}")
    return canonical_sha256(entries)


def _float_field(
    row: Mapping[str, str], aliases: Sequence[str]
) -> float | None:
    lowered = {key.strip().lower(): value for key, value in row.items()}
    for alias in aliases:
        raw = lowered.get(alias)
        if raw is None or raw.strip() in ("", "N/A", "nan"):
            continue
        try:
            value = float(raw)
        except ValueError as error:
            raise BenchmarkFailure(f"invalid numeric capture field {alias}: {raw}") from error
        if not math.isfinite(value):
            raise BenchmarkFailure(f"non-finite capture field {alias}: {raw}")
        return value
    return None


def audit_mangohud(package_root: Path) -> dict[str, Any]:
    files = [
        validate_sha256(
            package_root / relative,
            expected=expected,
            label=f"MangoHud {relative}",
        )
        for relative, expected in MANGOHUD_FILE_SHA256.items()
    ]
    version = _command_output(
        (str(package_root / "usr" / "bin" / "mangohud"), "--version")
    )
    if version != f"v{MANGOHUD_VERSION}":
        raise BenchmarkFailure(
            f"MangoHud version mismatch: expected v{MANGOHUD_VERSION}, got {version!r}"
        )
    return {
        "name": "MangoHud",
        "version": MANGOHUD_VERSION,
        "release_url": MANGOHUD_RELEASE_URL,
        "archive_sha256": MANGOHUD_ARCHIVE_SHA256,
        "files": files,
    }


def mangohud_environment(
    *,
    package_root: Path,
    output_dir: Path,
    api: str,
) -> tuple[dict[str, str], dict[str, Any]]:
    if api not in ("opengl", "vulkan"):
        raise BenchmarkFailure(f"unsupported MangoHud capture API: {api}")
    audit = audit_mangohud(package_root)
    output_dir.mkdir(parents=True, exist_ok=True)
    environment = {
        "MANGOHUD": "1",
        "MANGOHUD_CONFIG": (
            "no_display,log_duration=0,log_interval=0,"
            f"control={MANGOHUD_CONTROL_PREFIX}-%p,"
            f"output_folder={output_dir},permit_upload=0"
        ),
    }
    if api == "opengl":
        environment["LD_PRELOAD"] = str(
            package_root
            / "usr"
            / "lib"
            / "mangohud"
            / "lib64"
            / "libMangoHud_shim.so"
        )
    else:
        source = load_json_object(
            package_root
            / "usr"
            / "share"
            / "vulkan"
            / "implicit_layer.d"
            / "MangoHud.x86_64.json",
            "MangoHud Vulkan manifest",
        )
        source["layer"]["library_path"] = str(
            package_root
            / "usr"
            / "lib"
            / "mangohud"
            / "lib64"
            / "libMangoHud.so"
        )
        manifest_dir = output_dir / "vulkan-layer"
        manifest_dir.mkdir()
        manifest = manifest_dir / "MangoHud.x86_64.json"
        manifest.write_bytes(canonical_json_bytes(source))
        environment["VK_LAYER_PATH"] = str(manifest_dir)
        environment["VK_ADD_LAYER_PATH"] = str(manifest_dir)
        environment["VK_INSTANCE_LAYERS"] = "VK_LAYER_MANGOHUD_overlay_x86_64"
    return environment, audit


def mangohud_set_logging(
    *,
    process_pid: int,
    enabled: bool,
    timeout: float = 15.0,
    completion_dir: Path | None = None,
) -> dict[str, Any]:
    if process_pid <= 0:
        raise BenchmarkFailure("MangoHud control requires a positive process PID")
    socket_name = f"{MANGOHUD_CONTROL_PREFIX}-{process_pid}"
    deadline = time.monotonic() + timeout
    connection: socket.socket | None = None
    last_error: OSError | None = None
    while time.monotonic() < deadline:
        candidate = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        candidate.settimeout(1.0)
        try:
            candidate.connect("\0" + socket_name)
        except OSError as error:
            last_error = error
            candidate.close()
            time.sleep(0.1)
        else:
            connection = candidate
            break
    if connection is None:
        raise BenchmarkFailure(
            f"cannot connect to MangoHud control socket {socket_name}: {last_error}"
        )
    command = b":logging=1;" if enabled else b":logging=0;"
    with connection:
        connection.sendall(command)
        if enabled:
            return {
                "socket": socket_name,
                "command": "start-logging",
                "acknowledged": True,
            }
        response = bytearray()
        while time.monotonic() < deadline:
            try:
                chunk = connection.recv(4096)
            except socket.timeout:
                continue
            if not chunk:
                break
            response.extend(chunk)
            if b"LoggingFinished" in response:
                return {
                    "socket": socket_name,
                    "command": "stop-logging",
                    "acknowledged": True,
                }
    if completion_dir is not None:
        file_deadline = time.monotonic() + timeout
        while time.monotonic() < file_deadline:
            raw_logs = [
                path
                for path in completion_dir.glob("*.csv")
                if not path.name.endswith("_summary.csv")
            ]
            summaries = list(completion_dir.glob("*_summary.csv"))
            if raw_logs and summaries:
                return {
                    "socket": socket_name,
                    "command": "stop-logging",
                    "acknowledged": True,
                    "acknowledged_by": "closed raw and summary files",
                }
            time.sleep(0.1)
    raise BenchmarkFailure(
        f"MangoHud control socket {socket_name} did not acknowledge logging stop"
    )


def parse_mangohud_csv(path: Path) -> dict[str, list[float]]:
    try:
        lines = [
            line
            for line in path.read_text(encoding="utf-8-sig").splitlines()
            if line.strip() and not line.startswith("#")
        ]
    except OSError as error:
        raise BenchmarkFailure(f"cannot read MangoHud log {path}: {error}") from error
    header_index = next(
        (
            index
            for index, line in enumerate(lines)
            if {"fps", "frametime"}.issubset(
                {field.strip().lower() for field in line.split(",")}
            )
        ),
        None,
    )
    if header_index is None:
        raise BenchmarkFailure(f"MangoHud log has no frame-metrics header: {path}")
    reader = csv.DictReader(lines[header_index:])
    frames: list[float] = []
    gpu_load: list[float] = []
    gpu_clock: list[float] = []
    gpu_temp: list[float] = []
    for row in reader:
        frame_ms = _float_field(
            row, ("frametime", "frame_time", "frame_timing", "frametime_ms")
        )
        if frame_ms is None:
            fps = _float_field(row, ("fps",))
            if fps is None or fps <= 0.0:
                raise BenchmarkFailure("MangoHud row lacks positive frametime/FPS")
            frame_ms = 1000.0 / fps
        if frame_ms <= 0.0:
            raise BenchmarkFailure("MangoHud frame interval must be positive")
        frames.append(frame_ms)
        for output, aliases in (
            (gpu_load, ("gpu_load", "gpu load")),
            (gpu_clock, ("gpu_core_clock", "gpu_clock", "gpu clock")),
            (gpu_temp, ("gpu_temp", "gpu_temperature", "gpu temp")),
        ):
            value = _float_field(row, aliases)
            if value is not None:
                output.append(value)
    if not frames:
        raise BenchmarkFailure("MangoHud log contains no frame samples")
    return {
        "frame_ms": frames,
        "gpu_load": gpu_load,
        "gpu_clock_mhz": gpu_clock,
        "gpu_temp_c": gpu_temp,
    }


def percentile(values: Sequence[float], percent: float) -> float:
    if not values:
        raise BenchmarkFailure("cannot compute percentile of an empty sample")
    if not 0.0 <= percent <= 100.0:
        raise BenchmarkFailure(f"percentile is out of range: {percent}")
    ordered = sorted(float(value) for value in values)
    if len(ordered) == 1:
        return ordered[0]
    rank = (len(ordered) - 1) * percent / 100.0
    lower = math.floor(rank)
    upper = math.ceil(rank)
    if lower == upper:
        return ordered[lower]
    weight = rank - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def summarize_frames(
    capture: Mapping[str, Sequence[float]],
    *,
    hitch_thresholds_ms: Sequence[float] = DEFAULT_HITCH_THRESHOLDS_MS,
) -> FrameSummary:
    frames = list(capture.get("frame_ms", ()))
    if not frames:
        raise BenchmarkFailure("frame capture is empty")
    slowest_count = max(1, math.ceil(len(frames) * 0.01))
    slowest_mean = sum(sorted(frames, reverse=True)[:slowest_count]) / slowest_count
    mean_ms = sum(frames) / len(frames)

    def optional_mean(name: str) -> float | None:
        values = list(capture.get(name, ()))
        return sum(values) / len(values) if values else None

    return FrameSummary(
        samples=len(frames),
        p50_ms=percentile(frames, 50.0),
        p95_ms=percentile(frames, 95.0),
        p99_ms=percentile(frames, 99.0),
        one_percent_low_fps=1000.0 / slowest_mean,
        mean_fps=1000.0 / mean_ms,
        hitch_counts={
            f"ge_{threshold:g}_ms": sum(value >= threshold for value in frames)
            for threshold in hitch_thresholds_ms
        },
        gpu_load_mean=optional_mean("gpu_load"),
        gpu_clock_mean_mhz=optional_mean("gpu_clock_mhz"),
        gpu_temp_mean_c=optional_mean("gpu_temp_c"),
    )


def parse_perf_stat(path: Path, *, presented_frames: int) -> dict[str, Any]:
    if presented_frames <= 0:
        raise BenchmarkFailure("presented frame count must be positive")
    text = path.read_text(encoding="utf-8")
    if (
        "Access to performance monitoring" in text
        or "No permission to enable" in text
        or "perf_event_paranoid" in text
    ):
        raise BenchmarkFailure("perf stat lacks kernel permission")
    values: dict[str, float] = {}
    for raw_line in text.splitlines():
        if not raw_line.strip() or raw_line.lstrip().startswith("#"):
            continue
        fields = [field.strip() for field in raw_line.split(",")]
        if len(fields) < 3:
            continue
        raw_value, _unit, event = fields[:3]
        if event not in REQUIRED_PERF_EVENTS:
            continue
        if raw_value in ("<not counted>", "<not supported>"):
            raise BenchmarkFailure(f"perf event unavailable: {event}")
        try:
            values[event] = float(raw_value.replace(" ", ""))
        except ValueError as error:
            raise BenchmarkFailure(
                f"invalid perf stat value for {event}: {raw_value!r}"
            ) from error
    missing = [event for event in REQUIRED_PERF_EVENTS if event not in values]
    if missing:
        raise BenchmarkFailure(f"perf stat is missing events: {', '.join(missing)}")
    return {
        "events": values,
        "presented_frames": presented_frames,
        "per_presented_frame": {
            event: value / presented_frames for event, value in values.items()
        },
        "scope": "process totals and per-presented-frame means; no CPU-frame percentiles",
    }


_FTRACE_LINE = re.compile(
    r"^(?P<comm>.+?)-(?P<pid>\d+)\s+\[[^]]+\].*?\s"
    r"(?P<event>[a-zA-Z0-9_]+:[a-zA-Z0-9_]+):\s(?P<payload>.*)$"
)


def parse_ftrace(path: Path, *, client_pid: int) -> dict[str, Any]:
    if client_pid <= 0:
        raise BenchmarkFailure("client PID must be positive")
    counts = {event: 0 for event in REQUIRED_FTRACE_EVENTS}
    foreign_gpu_events = 0
    for line in path.read_text(encoding="utf-8").splitlines():
        match = _FTRACE_LINE.match(line)
        if match is None:
            continue
        event = match.group("event")
        if event not in counts:
            continue
        pid = int(match.group("pid"))
        payload = match.group("payload")
        associated = pid == client_pid or re.search(
            rf"\b(?:pid|tgid|context_pid)={client_pid}\b", payload
        )
        if associated:
            counts[event] += 1
        elif event.startswith(("amdgpu:", "drm:", "fence:")):
            foreign_gpu_events += 1
    missing = [event for event, count in counts.items() if count == 0]
    if missing:
        raise BenchmarkFailure(
            "ftrace cannot associate required events with the client PID: "
            + ", ".join(missing)
        )
    return {
        "client_pid": client_pid,
        "associated_events": counts,
        "foreign_gpu_events": foreign_gpu_events,
        "gpu_duration_claim_allowed": False,
        "scope": "event association only; submission-to-fence duration requires a proven context chain",
    }


def parse_server_camera_markers(
    log: str, replay: Mapping[str, Any]
) -> list[dict[str, Any]]:
    expected = [
        frame for frame in replay["frames"] if frame.get("marker") is not None
    ]
    parsed: list[dict[str, Any]] = []
    prefix = "BBB_BENCHMARK_MARKER "
    for line in log.splitlines():
        if prefix not in line:
            continue
        fields = line.split(prefix, 1)[1].split()
        if len(fields) != 9:
            raise BenchmarkFailure(f"malformed server camera marker: {line}")
        digest, segment, marker, raw_tick, *raw_pose = fields
        try:
            tick = int(raw_tick)
            pose = [float(value) for value in raw_pose]
        except ValueError as error:
            raise BenchmarkFailure(f"malformed server camera marker: {line}") from error
        parsed.append(
            {
                "replay_hash": digest,
                "capture_segment": segment,
                "marker": marker,
                "server_tick": tick,
                "x": pose[0],
                "y": pose[1],
                "z": pose[2],
                "yaw": pose[3],
                "pitch": pose[4],
            }
        )
    if len(parsed) != len(expected):
        raise BenchmarkFailure(
            "server camera marker count mismatch: "
            f"expected {len(expected)}, got {len(parsed)}"
        )
    digest = replay_hash(replay)
    for observed, frame in zip(parsed, expected, strict=True):
        for key in ("capture_segment", "marker", "server_tick"):
            if observed[key] != frame[key]:
                raise BenchmarkFailure(
                    f"server camera marker {key} mismatch at tick "
                    f"{frame['server_tick']}: {observed[key]!r}"
                )
        if observed["replay_hash"] != digest:
            raise BenchmarkFailure(
                f"server camera replay hash mismatch at tick {frame['server_tick']}"
            )
        for key in ("x", "y", "z", "yaw", "pitch"):
            if not math.isclose(
                observed[key], float(frame[key]), rel_tol=0.0, abs_tol=1.0e-6
            ):
                raise BenchmarkFailure(
                    f"server camera {key} mismatch at tick {frame['server_tick']}: "
                    f"{observed[key]} != {frame[key]}"
                )
    return parsed


def write_server_camera_log(
    path: Path, log: str, replay: Mapping[str, Any]
) -> dict[str, Any]:
    markers = parse_server_camera_markers(log, replay)
    with path.open("wb") as output:
        for marker in markers:
            output.write(canonical_json_bytes(marker))
    return {
        "path": str(path),
        "sha256": file_digest(path, "sha256"),
        "markers": len(markers),
    }


def validate_run(run: Mapping[str, Any], expected: Mapping[str, Any]) -> list[str]:
    reasons: list[str] = []
    for key in ("replay_hash", "world_hash", "resource_hash"):
        if run.get(key) != expected.get(key):
            reasons.append(f"{key} mismatch")
    if run.get("position_corrections") != 0:
        reasons.append("position correction observed")
    if run.get("resource_failures") not in (0, []):
        reasons.append("resource loading failure")
    if run.get("resolution") != [WINDOW_WIDTH, WINDOW_HEIGHT]:
        reasons.append("resolution mismatch")
    if run.get("settings_hash") != expected.get("settings_hash"):
        reasons.append("graphics settings mismatch")
    if run.get("visible_content_hash") != expected.get("visible_content_hash"):
        reasons.append("visible content mismatch")
    if run.get("capture_dropped_frames") != 0:
        reasons.append("capture hook dropped frames")
    if run.get("throttling") is not False:
        reasons.append("CPU/GPU throttling or unknown throttling state")
    if int(run.get("presented_frames", 0)) <= 0:
        reasons.append("no presented frames")
    if run.get("capture_api") not in ("opengl", "vulkan"):
        reasons.append("capture API is missing or unsupported")
    return reasons


class ProcessGroup:
    def __init__(self, argv: Sequence[str], **popen_options: Any) -> None:
        self.argv = tuple(argv)
        self.popen_options = popen_options
        self.process: subprocess.Popen[Any] | None = None

    def start(self) -> subprocess.Popen[Any]:
        if self.process is not None:
            raise BenchmarkFailure("process group was already started")
        self.process = subprocess.Popen(
            self.argv, start_new_session=True, **self.popen_options
        )
        return self.process

    def cleanup(self, timeout: float = 5.0) -> None:
        process = self.process
        if process is None or process.poll() is not None:
            return
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            return
        try:
            process.wait(timeout=timeout)
            return
        except subprocess.TimeoutExpired:
            pass
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            return
        process.wait(timeout=timeout)

    def kill(self, timeout: float = 5.0) -> None:
        process = self.process
        if process is None or process.poll() is not None:
            return
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            return
        process.wait(timeout=timeout)

    def __enter__(self) -> "ProcessGroup":
        self.start()
        return self

    def __exit__(self, *_: Any) -> None:
        self.cleanup()


def _command_output(argv: Sequence[str]) -> str:
    try:
        result = subprocess.run(
            argv,
            check=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=15.0,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"unavailable: {error}"
    return result.stdout.strip()


def host_facts(java_path: Path) -> dict[str, Any]:
    drm_devices: list[dict[str, str]] = []
    for device in sorted(Path("/sys/class/drm").glob("card[0-9]*")):
        if re.fullmatch(r"card[0-9]+", device.name) is None:
            continue
        facts = {"device": device.name}
        for name in ("vendor", "device"):
            path = device / "device" / name
            if path.is_file():
                facts[name] = path.read_text(encoding="utf-8").strip()
        driver = device / "device" / "driver"
        if driver.exists():
            facts["driver"] = Path(os.path.realpath(driver)).name
        drm_devices.append(facts)
    mesa = _command_output(("glxinfo", "-B")) if shutil.which("glxinfo") else "glxinfo unavailable"
    vulkan = (
        _command_output(("vulkaninfo", "--summary"))
        if shutil.which("vulkaninfo")
        else "vulkaninfo unavailable"
    )
    mesa_packages = (
        _command_output(
            (
                "dpkg-query",
                "-W",
                "-f=${Package} ${Version}\\n",
                "libegl-mesa0",
                "libgl1-mesa-dri",
                "mesa-vulkan-drivers",
            )
        )
        if shutil.which("dpkg-query")
        else "dpkg-query unavailable"
    )
    pci_display = (
        _command_output(("lspci", "-nnk"))
        if shutil.which("lspci")
        else "lspci unavailable"
    )
    return {
        "schema": BENCHMARK_SCHEMA,
        "kernel": platform.uname()._asdict(),
        "java": _command_output((str(java_path), "-version")),
        "python": sys.version,
        "drm": drm_devices,
        "mesa_glx": mesa,
        "mesa_packages": mesa_packages,
        "pci": pci_display,
        "vulkan": vulkan,
        "wayland_display": os.environ.get("WAYLAND_DISPLAY"),
        "xdg_runtime_dir": os.environ.get("XDG_RUNTIME_DIR"),
    }


def capture_preflight(
    *, java_path: Path, mangohud_root: Path | None
) -> dict[str, Any]:
    blockers: list[dict[str, str]] = []
    render_nodes: list[dict[str, Any]] = []
    for path in sorted(Path("/dev/dri").glob("renderD*")):
        status = path.stat()
        entry: dict[str, Any] = {
            "path": str(path),
            "mode": stat.filemode(status.st_mode),
            "uid": status.st_uid,
            "gid": status.st_gid,
            "owner": pwd.getpwuid(status.st_uid).pw_name,
            "group": grp.getgrgid(status.st_gid).gr_name,
            "readable": os.access(path, os.R_OK),
            "writable": os.access(path, os.W_OK),
            "open_read_write": False,
        }
        try:
            descriptor = os.open(path, os.O_RDWR | os.O_CLOEXEC)
        except OSError as error:
            entry["open_error"] = f"{type(error).__name__}: {error}"
        else:
            os.close(descriptor)
            entry["open_read_write"] = True
        render_nodes.append(entry)
    if not render_nodes or not any(
        node["open_read_write"] for node in render_nodes
    ):
        blockers.append(
            {
                "code": "render_node_unavailable",
                "detail": "the benchmark user cannot open any DRM render node read-write",
            }
        )

    perf_argv = (
        "perf",
        "stat",
        "-x,",
        "-e",
        ",".join(REQUIRED_PERF_EVENTS),
        "--",
        "/bin/true",
    )
    try:
        perf_result = subprocess.run(
            perf_argv,
            check=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=20.0,
        )
        perf_status = {
            "argv": list(perf_argv),
            "returncode": perf_result.returncode,
            "output": perf_result.stdout,
        }
    except (OSError, subprocess.TimeoutExpired) as error:
        perf_status = {
            "argv": list(perf_argv),
            "returncode": None,
            "output": f"{type(error).__name__}: {error}",
        }
    if perf_status["returncode"] != 0:
        blockers.append(
            {
                "code": "perf_events_unavailable",
                "detail": "perf stat cannot open the required process counters",
            }
        )

    trace_roots: list[Path] = []
    trace_root_errors: dict[str, str] = {}
    for path in (Path("/sys/kernel/tracing"), Path("/sys/kernel/debug/tracing")):
        try:
            path.stat()
        except FileNotFoundError:
            continue
        except OSError as error:
            trace_root_errors[str(path)] = f"{type(error).__name__}: {error}"
        else:
            trace_roots.append(path)
    trace_events: dict[str, dict[str, Any]] = {}
    for event in REQUIRED_FTRACE_EVENTS:
        group, name = event.split(":", 1)
        candidates = [
            root / "events" / group / name / "enable" for root in trace_roots
        ]
        trace_events[event] = {
            "paths": [str(path) for path in candidates],
            "readable": any(os.access(path, os.R_OK) for path in candidates),
            "writable": any(os.access(path, os.W_OK) for path in candidates),
        }
    trace_access = bool(trace_roots) and all(
        event["readable"] and event["writable"]
        for event in trace_events.values()
    )
    if not trace_access:
        blockers.append(
            {
                "code": "tracefs_events_unavailable",
                "detail": "required amdgpu/fence/drm/sched trace events are not user-readable and writable",
            }
        )

    mango: dict[str, Any] | None = None
    if mangohud_root is not None:
        mango = audit_mangohud(mangohud_root)
    else:
        blockers.append(
            {
                "code": "mangohud_not_provisioned",
                "detail": "no fixed MangoHud package was supplied",
            }
        )
    paranoid_path = Path("/proc/sys/kernel/perf_event_paranoid")
    return {
        "schema": BENCHMARK_SCHEMA,
        "host": host_facts(java_path),
        "identity": {
            "uid": os.getuid(),
            "gid": os.getgid(),
            "groups": [
                {"gid": group, "name": grp.getgrgid(group).gr_name}
                for group in os.getgroups()
            ],
        },
        "render_nodes": render_nodes,
        "perf_event_paranoid": paranoid_path.read_text(encoding="utf-8").strip()
        if paranoid_path.is_file()
        else None,
        "perf": perf_status,
        "trace_roots": [str(path) for path in trace_roots],
        "trace_root_errors": trace_root_errors,
        "trace_events": trace_events,
        "mangohud": mango,
        "tools": {
            name: {
                "path": shutil.which(name),
                "version": _command_output((name, "--version"))
                if shutil.which(name)
                else "unavailable",
            }
            for name in ("perf", "weston", "weston-screenshooter", "renderdoccmd")
        },
        "blockers": blockers,
        "formal_capture_ready": not blockers,
        "gpu_duration_claim_allowed": False,
    }


def summarize_blocked_comparison(
    *,
    preflight: Mapping[str, Any],
    vanilla: Mapping[str, Any],
    bbb: Mapping[str, Any],
    visible_content_status: str,
    visible_content_note: str,
) -> dict[str, Any]:
    if visible_content_status not in ("match", "mismatch", "unverified"):
        raise BenchmarkFailure(
            f"invalid visible-content review status: {visible_content_status}"
        )
    for name, smoke, expected_client in (
        ("vanilla", vanilla, "vanilla"),
        ("bbb", bbb, "bbb"),
    ):
        if smoke.get("client") != expected_client:
            raise BenchmarkFailure(f"{name} smoke client identity mismatch")
    contract_keys = (
        "replay_hash",
        "world_hash",
        "resource_hash",
        "settings_hash",
    )
    contract_matches = {
        key: vanilla.get(key) == bbb.get(key) and vanilla.get(key) is not None
        for key in contract_keys
    }
    vanilla_camera = vanilla.get("server_camera_log")
    bbb_camera = bbb.get("server_camera_log")
    camera_match = (
        isinstance(vanilla_camera, Mapping)
        and isinstance(bbb_camera, Mapping)
        and vanilla_camera.get("sha256") == bbb_camera.get("sha256")
        and vanilla_camera.get("markers") == bbb_camera.get("markers")
    )
    blockers = list(preflight.get("blockers", ()))
    if visible_content_status != "match":
        blockers.append(
            {
                "code": (
                    "visible_content_mismatch"
                    if visible_content_status == "mismatch"
                    else "visible_content_unverified"
                ),
                "detail": visible_content_note,
            }
        )
    tools = preflight.get("tools")
    renderdoc = tools.get("renderdoccmd") if isinstance(tools, Mapping) else None
    if not isinstance(renderdoc, Mapping) or not renderdoc.get("path"):
        blockers.append(
            {
                "code": "official_draw_evidence_unavailable",
                "detail": (
                    "RenderDoc is unavailable and no API-neutral official-client "
                    "draw/visible-count capture was produced"
                ),
            }
        )
    if any(
        smoke.get("client_cleanup", {}).get("signal") == "SIGKILL"
        for smoke in (vanilla, bbb)
        if isinstance(smoke.get("client_cleanup"), Mapping)
    ):
        blockers.append(
            {
                "code": "mangohud_wayland_teardown_unstable",
                "detail": (
                    "MangoHud 0.8.4 capture closes successfully, but both clients "
                    "require post-capture SIGKILL to avoid reproducible Wayland teardown crashes"
                ),
            }
        )

    screenshot_pairs = []
    vanilla_screenshots = {
        screenshot.get("capture_segment"): screenshot
        for screenshot in vanilla.get("screenshots", ())
        if isinstance(screenshot, Mapping)
    }
    bbb_screenshots = {
        screenshot.get("capture_segment"): screenshot
        for screenshot in bbb.get("screenshots", ())
        if isinstance(screenshot, Mapping)
    }
    for segment in sorted(set(vanilla_screenshots) | set(bbb_screenshots)):
        screenshot_pairs.append(
            {
                "capture_segment": segment,
                "vanilla": vanilla_screenshots.get(segment),
                "bbb": bbb_screenshots.get(segment),
            }
        )
    return {
        "schema": BENCHMARK_SCHEMA,
        "formal_comparison_complete": False,
        "metrics_publishable": False,
        "metric_scope": (
            "MangoHud end-to-end presented-frame smoke intervals only; "
            "not CPU render duration, not GPU frame duration, and not a formal comparison"
        ),
        "contract_matches": contract_matches,
        "server_camera_log_matches": camera_match,
        "visible_content_review": {
            "status": visible_content_status,
            "note": visible_content_note,
            "screenshot_pairs": screenshot_pairs,
        },
        "smoke_metrics": {
            "vanilla": vanilla.get("mangohud_frame_summaries"),
            "bbb": bbb.get("mangohud_frame_summaries"),
        },
        "artifacts": {
            "preflight": preflight.get("artifact_path"),
            "vanilla_smoke": vanilla.get("artifact_path"),
            "bbb_smoke": bbb.get("artifact_path"),
            "vanilla_raw": vanilla.get("mangohud_raw_logs"),
            "bbb_raw": bbb.get("mangohud_raw_logs"),
        },
        "blockers": blockers,
    }


def _verify_world_snapshot(
    manifest_path: Path, world_dir: Path
) -> dict[str, Any]:
    manifest = load_json_object(manifest_path, "world snapshot manifest")
    if manifest.get("schema") != BENCHMARK_SCHEMA:
        raise BenchmarkFailure("world snapshot manifest schema mismatch")
    if manifest.get("minecraft_version") != MINECRAFT_VERSION:
        raise BenchmarkFailure("world snapshot manifest version mismatch")
    actual_hash = hash_tree(world_dir, ignored_names=("session.lock",))
    if actual_hash != manifest.get("world_hash"):
        raise BenchmarkFailure(
            "world snapshot hash mismatch: "
            f"expected {manifest.get('world_hash')}, got {actual_hash}"
        )
    return manifest


def _take_weston_screenshot(
    *,
    artifact_dir: Path,
    environment: Mapping[str, str],
    client: str,
    segment: str,
    tick: int,
) -> dict[str, Any]:
    shooter = shutil.which("weston-screenshooter")
    if shooter is None:
        raise BenchmarkFailure("weston-screenshooter is unavailable")
    before = set(artifact_dir.glob("wayland-screenshot*.png"))
    capture = subprocess.run(
        (shooter,),
        cwd=artifact_dir,
        env=dict(environment),
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=20.0,
        check=False,
    )
    created = sorted(
        set(artifact_dir.glob("wayland-screenshot*.png")) - before,
        key=lambda path: path.stat().st_mtime_ns,
    )
    if capture.returncode != 0 or not created:
        raise BenchmarkFailure(
            "Weston screenshot failed: "
            + (capture.stdout.strip() or f"exit {capture.returncode}")
        )
    generated = created[-1]
    destination = artifact_dir / f"{client}-{segment}-tick-{tick}.png"
    generated.rename(destination)
    return {
        "path": str(destination),
        "sha256": file_digest(destination, "sha256"),
        "size": destination.stat().st_size,
        "capture_segment": segment,
        "server_tick": tick,
        "compositor_resolution": [COMPOSITOR_WIDTH, COMPOSITOR_HEIGHT],
        "client_resolution": [WINDOW_WIDTH, WINDOW_HEIGHT],
    }


def run_official_smoke(
    *,
    replay: Mapping[str, Any],
    mc_root: Path,
    java_path: Path,
    world_manifest_path: Path,
    artifact_dir: Path,
    server_port: int,
    accept_eula: bool,
    timeout: float,
    wayland_display: str,
    mangohud_root: Path | None,
    weston_path: Path | None,
) -> dict[str, Any]:
    if artifact_dir.exists() and any(artifact_dir.iterdir()):
        raise BenchmarkFailure(
            f"official smoke artifact directory must be empty: {artifact_dir}"
        )
    artifact_dir.mkdir(parents=True, exist_ok=True)
    source_manifest = load_json_object(
        world_manifest_path, "world snapshot manifest"
    )
    source_world = Path(str(source_manifest.get("world", "")))
    _verify_world_snapshot(world_manifest_path, source_world)

    server_dir = artifact_dir / "server"
    shutil.copytree(source_world, server_dir / "world")
    digest = write_server_layout(
        server_dir,
        replay=replay,
        server_port=server_port,
        accept_eula=accept_eula,
    )
    if digest != source_manifest.get("replay_hash"):
        raise BenchmarkFailure("world snapshot and requested replay hash differ")

    audit = audit_distribution(mc_root, java_path=java_path)
    server = validate_server_jar(mc_root)
    natives_dir = artifact_dir / "natives"
    natives = safe_extract_natives(
        (Path(entry.path) for entry in audit.native_classifiers), natives_dir
    )
    game_dir = artifact_dir / "game"
    options_path = write_vanilla_options(game_dir)
    plan = build_vanilla_launch_plan(
        audit,
        java_path=java_path,
        game_dir=game_dir,
        assets_dir=mc_root / "downloads" / MINECRAFT_VERSION / "assets",
        natives_dir=natives_dir,
        server_port=server_port,
    )
    server_log_path = artifact_dir / "server.log"
    client_log_path = artifact_dir / "vanilla-client.log"
    evidence_path = artifact_dir / "smoke.json"
    launch_path = artifact_dir / "vanilla-launch.json"
    host = host_facts(java_path)
    host["wayland_display"] = wayland_display
    launch_path.write_bytes(
        canonical_json_bytes(
            {
                "schema": BENCHMARK_SCHEMA,
                "audit": asdict(audit),
                "natives": [asdict(entry) for entry in natives],
                "options_hash": file_digest(options_path, "sha256"),
                "launch": {**asdict(plan), "argv": sanitized_argv(plan.argv)},
                "host": host,
            }
        )
    )

    result: dict[str, Any] = {
        "schema": BENCHMARK_SCHEMA,
        "client": "vanilla",
        "status": "started",
        "replay_hash": digest,
        "world_hash": source_manifest["world_hash"],
        "resource_hash": audit.resource_hash,
        "settings_hash": canonical_sha256(equal_load_settings()),
        "scene_contract_hash": canonical_sha256(
            {
                "replay_hash": digest,
                "world_hash": source_manifest["world_hash"],
                "resource_hash": audit.resource_hash,
            }
        ),
        "visible_content_hash": None,
        "formal_capture_valid": False,
        "scope": "launcher/replay/common-capture semantic smoke, not a formal performance run",
        "server": asdict(server),
        "server_port": server_port,
        "launch_manifest": str(launch_path),
        "server_log": str(server_log_path),
        "client_log": str(client_log_path),
        "screenshot": None,
        "screenshots": [],
        "wayland_display": wayland_display,
        "resolution": [WINDOW_WIDTH, WINDOW_HEIGHT],
    }
    server_group: ProcessGroup | None = None
    client_group: ProcessGroup | None = None
    compositor_group: ProcessGroup | None = None
    compositor_output: Any = None
    try:
        if re.fullmatch(r"[A-Za-z0-9_.-]+", wayland_display) is None:
            raise BenchmarkFailure(
                f"unsafe Wayland display name: {wayland_display!r}"
            )
        runtime_dir = Path(os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000"))
        if weston_path is not None:
            if not weston_path.is_file() or not os.access(weston_path, os.X_OK):
                raise BenchmarkFailure(f"Weston executable is unavailable: {weston_path}")
            socket_path = runtime_dir / wayland_display
            if socket_path.exists():
                raise BenchmarkFailure(
                    f"refusing to replace existing Wayland socket: {socket_path}"
                )
            compositor_output = (artifact_dir / "weston-stdout.log").open(
                "w", encoding="utf-8"
            )
            compositor_argv = (
                str(weston_path),
                "--backend=headless-backend.so",
                f"--socket={wayland_display}",
                "--idle-time=0",
                f"--width={COMPOSITOR_WIDTH}",
                f"--height={COMPOSITOR_HEIGHT}",
                "--renderer=pixman",
                "--no-config",
                "--debug",
                f"--log={artifact_dir / 'weston.log'}",
            )
            compositor_group = ProcessGroup(
                compositor_argv,
                stdin=subprocess.DEVNULL,
                stdout=compositor_output,
                stderr=subprocess.STDOUT,
                env={**os.environ, "XDG_RUNTIME_DIR": str(runtime_dir)},
            )
            compositor_process = compositor_group.start()
            _wait_for_socket(
                socket_path,
                process=compositor_process,
                timeout=15.0,
            )
            result["compositor"] = {
                "version": _command_output((str(weston_path), "--version")),
                "argv": list(compositor_argv),
                "log": str(artifact_dir / "weston.log"),
            }
        with (
            server_log_path.open("w", encoding="utf-8") as server_log,
            client_log_path.open("w", encoding="utf-8") as client_log,
        ):
            server_group = ProcessGroup(
                (
                    str(java_path),
                    "-Xms1G",
                    "-Xmx2G",
                    "-jar",
                    server.path,
                    "nogui",
                ),
                cwd=server_dir,
                stdin=subprocess.PIPE,
                stdout=server_log,
                stderr=subprocess.STDOUT,
                text=True,
            )
            server_process = server_group.start()
            _wait_for_log(
                server_log_path,
                SERVER_READY_MARKER,
                process=server_process,
                timeout=timeout,
            )
            _write_server_commands(
                server_process,
                (
                    "scoreboard players set #run bbb_replay_control 0",
                    "time set noon",
                    "weather clear 1000000",
                ),
            )
            environment = os.environ.copy()
            environment["XDG_RUNTIME_DIR"] = str(runtime_dir)
            environment["WAYLAND_DISPLAY"] = wayland_display
            if mangohud_root is not None:
                mango_environment, mango_audit = mangohud_environment(
                    package_root=mangohud_root,
                    output_dir=artifact_dir / "mangohud",
                    api="opengl",
                )
                environment.update(mango_environment)
                result["mangohud"] = mango_audit
            client_group = ProcessGroup(
                plan.argv,
                cwd=game_dir,
                stdin=subprocess.DEVNULL,
                stdout=client_log,
                stderr=subprocess.STDOUT,
                env=environment,
            )
            client_process = client_group.start()
            result["client_pid"] = client_process.pid
            _wait_for_log(
                server_log_path,
                f"{BENCHMARK_USERNAME} joined the game",
                process=client_process,
                timeout=timeout,
            )
            result["warmup_seconds"] = FIXED_WARMUP_SECONDS
            _warmup_and_start_replay(server_process, replay)
            if mangohud_root is not None:
                capture_start = next(
                    frame
                    for frame in replay["frames"]
                    if frame.get("marker") == "capture_start"
                )
                _wait_for_log(
                    server_log_path,
                    (
                        f"BBB_BENCHMARK_MARKER {digest} "
                        f"{capture_start['capture_segment']} capture_start "
                        f"{capture_start['server_tick']}"
                    ),
                    process=client_process,
                    timeout=timeout,
                )
                result["mangohud_capture_start"] = mangohud_set_logging(
                    process_pid=client_process.pid, enabled=True
                )
            for anchor in (
                frame
                for frame in replay["frames"]
                if frame.get("marker") == "anchor"
            ):
                anchor_marker = (
                    f"BBB_BENCHMARK_MARKER {digest} "
                    f"{anchor['capture_segment']} anchor {anchor['server_tick']}"
                )
                _wait_for_log(
                    server_log_path,
                    anchor_marker,
                    process=client_process,
                    timeout=timeout,
                )
                time.sleep(0.5)
                if weston_path is not None:
                    result["screenshots"].append(
                        _take_weston_screenshot(
                            artifact_dir=artifact_dir,
                            environment=environment,
                            client="vanilla",
                            segment=anchor["capture_segment"],
                            tick=anchor["server_tick"],
                        )
                    )
            final_frame = replay["frames"][-1]
            final_marker = (
                f"BBB_BENCHMARK_MARKER {digest} "
                f"{final_frame['capture_segment']} "
                f"{final_frame['marker']} {final_frame['server_tick']}"
            )
            _wait_for_log(
                server_log_path,
                final_marker,
                process=client_process,
                timeout=timeout,
            )
            if mangohud_root is not None:
                result["mangohud_capture_stop"] = mangohud_set_logging(
                    process_pid=client_process.pid,
                    enabled=False,
                    completion_dir=artifact_dir / "mangohud",
                )
            if result["screenshots"]:
                result["screenshot"] = result["screenshots"][-1]
            result["status"] = "passed"
    except BaseException as error:
        result["status"] = (
            "interrupted" if isinstance(error, KeyboardInterrupt) else "failed"
        )
        result["failure"] = str(error)
        raise
    finally:
        if client_group is not None:
            if mangohud_root is not None:
                result["client_cleanup"] = {
                    "signal": "SIGKILL",
                    "reason": (
                        "MangoHud 0.8.4 OpenGL on native Wayland reproducibly "
                        "crashes in wl_proxy_destroy during SIGTERM teardown; "
                        "the control socket has already acknowledged logging stop"
                    ),
                }
                client_group.kill(timeout=10.0)
            else:
                result["client_cleanup"] = {"signal": "SIGTERM"}
                client_group.cleanup(timeout=10.0)
            if client_group.process is not None:
                result["client_exit_status"] = client_group.process.returncode
        if server_group is not None:
            process = server_group.process
            if process is not None and process.poll() is None:
                try:
                    _write_server_commands(process, ("stop",))
                    process.wait(timeout=15.0)
                except (BenchmarkFailure, subprocess.TimeoutExpired):
                    pass
            server_group.cleanup(timeout=10.0)
        if compositor_group is not None:
            compositor_group.cleanup(timeout=10.0)
        if compositor_output is not None:
            compositor_output.close()
        if server_log_path.is_file():
            log = server_log_path.read_text(encoding="utf-8", errors="replace")
            result["marker_lines"] = [
                line for line in log.splitlines() if "BBB_BENCHMARK_" in line
            ]
            result["position_corrections"] = log.count(
                "moved too quickly"
            ) + log.count("moved wrongly")
            try:
                result["server_camera_log"] = write_server_camera_log(
                    artifact_dir / "server-camera.jsonl", log, replay
                )
            except BenchmarkFailure as error:
                result.setdefault("invalid_reasons", []).append(str(error))
        crash_files = sorted(game_dir.glob("hs_err_pid*.log"))
        result["jvm_crashes"] = [
            {
                "path": str(path),
                "size": path.stat().st_size,
                "sha256": file_digest(path, "sha256"),
            }
            for path in crash_files
        ]
        if crash_files:
            result.setdefault("invalid_reasons", []).append(
                "official client JVM crash report was produced during capture cleanup"
            )
        if options_path.is_file():
            try:
                result["loaded_options"] = audit_vanilla_options(options_path)
            except BenchmarkFailure as error:
                result.setdefault("invalid_reasons", []).append(str(error))
        mango_dir = artifact_dir / "mangohud"
        if mango_dir.is_dir():
            result["mangohud_artifacts"] = [
                {
                    "path": str(path),
                    "size": path.stat().st_size,
                    "sha256": file_digest(path, "sha256"),
                }
                for path in sorted(mango_dir.rglob("*"))
                if path.is_file()
            ]
            raw_logs = sorted(
                path
                for path in mango_dir.rglob("*.csv")
                if not path.name.endswith("_summary.csv")
            )
            result["mangohud_raw_logs"] = [str(path) for path in raw_logs]
            client_log = (
                client_log_path.read_text(encoding="utf-8", errors="replace")
                if client_log_path.is_file()
                else ""
            )
            result["mangohud_hook_initialized"] = "[MANGOHUD]" in client_log
            if not result["mangohud_hook_initialized"]:
                result.setdefault("invalid_reasons", []).append(
                    "MangoHud OpenGL hook did not report initialization"
                )
            if not raw_logs:
                result.setdefault("invalid_reasons", []).append(
                    "MangoHud produced no raw per-frame CSV"
                )
            else:
                try:
                    result["mangohud_frame_summaries"] = [
                        asdict(summarize_frames(parse_mangohud_csv(path)))
                        for path in raw_logs
                    ]
                except BenchmarkFailure as error:
                    result.setdefault("invalid_reasons", []).append(str(error))
        if result.get("invalid_reasons") and result["status"] == "passed":
            result["status"] = "invalid"
        evidence_path.write_bytes(canonical_json_bytes(result))
    return result


def _control_request(
    address: tuple[str, int], method: str, params: Any = None
) -> Any:
    request = canonical_json_bytes({"method": method, "params": params})
    with socket.create_connection(address, timeout=10.0) as connection:
        connection.settimeout(60.0)
        connection.sendall(request)
        response = bytearray()
        while not response.endswith(b"\n"):
            chunk = connection.recv(65536)
            if not chunk:
                break
            response.extend(chunk)
    if not response:
        raise BenchmarkFailure(f"bbb-control method {method} returned no response")
    try:
        decoded = json.loads(response)
    except json.JSONDecodeError as error:
        raise BenchmarkFailure(
            f"bbb-control method {method} returned invalid JSON"
        ) from error
    if decoded.get("ok") is not True:
        raise BenchmarkFailure(
            f"bbb-control method {method} failed: {decoded.get('error')}"
        )
    return decoded.get("result")


def run_bbb_smoke(
    *,
    replay: Mapping[str, Any],
    mc_root: Path,
    java_path: Path,
    bbb_binary: Path,
    world_manifest_path: Path,
    artifact_dir: Path,
    server_port: int,
    control_port: int,
    accept_eula: bool,
    timeout: float,
    wayland_display: str,
    mangohud_root: Path | None,
    weston_path: Path,
) -> dict[str, Any]:
    if artifact_dir.exists() and any(artifact_dir.iterdir()):
        raise BenchmarkFailure(f"bbb smoke artifact directory must be empty: {artifact_dir}")
    artifact_dir.mkdir(parents=True, exist_ok=True)
    source_manifest = load_json_object(
        world_manifest_path, "world snapshot manifest"
    )
    source_world = Path(str(source_manifest.get("world", "")))
    _verify_world_snapshot(world_manifest_path, source_world)
    server_dir = artifact_dir / "server"
    shutil.copytree(source_world, server_dir / "world")
    digest = write_server_layout(
        server_dir,
        replay=replay,
        server_port=server_port,
        accept_eula=accept_eula,
    )
    if digest != source_manifest.get("replay_hash"):
        raise BenchmarkFailure("world snapshot and requested replay hash differ")
    audit = audit_distribution(mc_root, java_path=java_path)
    server = validate_server_jar(mc_root)
    client_argv = build_bbb_launch_argv(
        binary=bbb_binary,
        server_port=server_port,
        control_port=control_port,
    )
    runtime_dir = Path(os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000"))
    if re.fullmatch(r"[A-Za-z0-9_.-]+", wayland_display) is None:
        raise BenchmarkFailure(f"unsafe Wayland display name: {wayland_display!r}")
    if not weston_path.is_file() or not os.access(weston_path, os.X_OK):
        raise BenchmarkFailure(f"Weston executable is unavailable: {weston_path}")
    socket_path = runtime_dir / wayland_display
    if socket_path.exists():
        raise BenchmarkFailure(f"refusing to replace existing Wayland socket: {socket_path}")

    server_log_path = artifact_dir / "server.log"
    client_log_path = artifact_dir / "bbb-client.log"
    evidence_path = artifact_dir / "smoke.json"
    launch_path = artifact_dir / "bbb-launch.json"
    settings_hash = canonical_sha256(equal_load_settings())
    result: dict[str, Any] = {
        "schema": BENCHMARK_SCHEMA,
        "client": "bbb",
        "status": "started",
        "replay_hash": digest,
        "world_hash": source_manifest["world_hash"],
        "resource_hash": audit.resource_hash,
        "settings_hash": settings_hash,
        "scene_contract_hash": canonical_sha256(
            {
                "replay_hash": digest,
                "world_hash": source_manifest["world_hash"],
                "resource_hash": audit.resource_hash,
            }
        ),
        "visible_content_hash": None,
        "formal_capture_valid": False,
        "scope": "launcher/replay/common-capture semantic smoke, not a formal performance run",
        "server": asdict(server),
        "server_port": server_port,
        "control_port": control_port,
        "server_log": str(server_log_path),
        "client_log": str(client_log_path),
        "resolution": [WINDOW_WIDTH, WINDOW_HEIGHT],
        "wayland_display": wayland_display,
        "screenshots": [],
        "anchor_observations": [],
    }
    server_group: ProcessGroup | None = None
    client_group: ProcessGroup | None = None
    compositor_group: ProcessGroup | None = None
    compositor_output: Any = None
    try:
        compositor_output = (artifact_dir / "weston-stdout.log").open(
            "w", encoding="utf-8"
        )
        compositor_argv = (
            str(weston_path),
            "--backend=headless-backend.so",
            f"--socket={wayland_display}",
            "--idle-time=0",
            f"--width={COMPOSITOR_WIDTH}",
            f"--height={COMPOSITOR_HEIGHT}",
            "--renderer=pixman",
            "--no-config",
            "--debug",
            f"--log={artifact_dir / 'weston.log'}",
        )
        compositor_group = ProcessGroup(
            compositor_argv,
            stdin=subprocess.DEVNULL,
            stdout=compositor_output,
            stderr=subprocess.STDOUT,
            env={**os.environ, "XDG_RUNTIME_DIR": str(runtime_dir)},
        )
        compositor_process = compositor_group.start()
        _wait_for_socket(socket_path, process=compositor_process, timeout=15.0)
        result["compositor"] = {
            "version": _command_output((str(weston_path), "--version")),
            "argv": list(compositor_argv),
            "log": str(artifact_dir / "weston.log"),
        }
        environment = os.environ.copy()
        environment.update(
            {
                "XDG_RUNTIME_DIR": str(runtime_dir),
                "WAYLAND_DISPLAY": wayland_display,
                "BBB_MC_CODE_ROOT": str(mc_root),
                "RUST_LOG": "bbb_net=info,bbb_native=info,bbb_control=info",
            }
        )
        if mangohud_root is not None:
            mango_environment, mango_audit = mangohud_environment(
                package_root=mangohud_root,
                output_dir=artifact_dir / "mangohud",
                api="vulkan",
            )
            environment.update(mango_environment)
            result["mangohud"] = mango_audit
        launch_path.write_bytes(
            canonical_json_bytes(
                {
                    "schema": BENCHMARK_SCHEMA,
                    "binary": {
                        "path": str(bbb_binary),
                        "size": bbb_binary.stat().st_size,
                        "sha256": file_digest(bbb_binary, "sha256"),
                    },
                    "argv": list(client_argv),
                    "environment": {
                        key: environment[key]
                        for key in (
                            "BBB_MC_CODE_ROOT",
                            "WAYLAND_DISPLAY",
                            "XDG_RUNTIME_DIR",
                            "MANGOHUD",
                            "MANGOHUD_CONFIG",
                            "VK_LAYER_PATH",
                        )
                        if key in environment
                    },
                    "settings": equal_load_settings(),
                    "host": host_facts(java_path),
                }
            )
        )
        result["launch_manifest"] = str(launch_path)
        with (
            server_log_path.open("w", encoding="utf-8") as server_log,
            client_log_path.open("w", encoding="utf-8") as client_log,
        ):
            server_group = ProcessGroup(
                (
                    str(java_path),
                    "-Xms1G",
                    "-Xmx2G",
                    "-jar",
                    server.path,
                    "nogui",
                ),
                cwd=server_dir,
                stdin=subprocess.PIPE,
                stdout=server_log,
                stderr=subprocess.STDOUT,
                text=True,
            )
            server_process = server_group.start()
            _wait_for_log(
                server_log_path,
                SERVER_READY_MARKER,
                process=server_process,
                timeout=timeout,
            )
            _write_server_commands(
                server_process,
                (
                    "scoreboard players set #run bbb_replay_control 0",
                    "time set noon",
                    "weather clear 1000000",
                ),
            )
            client_group = ProcessGroup(
                client_argv,
                cwd=artifact_dir,
                stdin=subprocess.DEVNULL,
                stdout=client_log,
                stderr=subprocess.STDOUT,
                env=environment,
            )
            client_process = client_group.start()
            result["client_pid"] = client_process.pid
            _wait_for_log(
                server_log_path,
                f"{BENCHMARK_USERNAME} joined the game",
                process=client_process,
                timeout=timeout,
            )
            result["warmup_seconds"] = FIXED_WARMUP_SECONDS
            _warmup_and_start_replay(server_process, replay)
            if mangohud_root is not None:
                capture_start = next(
                    frame
                    for frame in replay["frames"]
                    if frame.get("marker") == "capture_start"
                )
                _wait_for_log(
                    server_log_path,
                    (
                        f"BBB_BENCHMARK_MARKER {digest} "
                        f"{capture_start['capture_segment']} capture_start "
                        f"{capture_start['server_tick']}"
                    ),
                    process=client_process,
                    timeout=timeout,
                )
                result["mangohud_capture_start"] = mangohud_set_logging(
                    process_pid=client_process.pid, enabled=True
                )
            for anchor in (
                frame
                for frame in replay["frames"]
                if frame.get("marker") == "anchor"
            ):
                _wait_for_log(
                    server_log_path,
                    (
                        f"BBB_BENCHMARK_MARKER {digest} "
                        f"{anchor['capture_segment']} anchor {anchor['server_tick']}"
                    ),
                    process=client_process,
                    timeout=timeout,
                )
                time.sleep(0.5)
                result["anchor_observations"].append(
                    {
                        "capture_segment": anchor["capture_segment"],
                        "server_tick": anchor["server_tick"],
                        "renderer": _control_request(
                            ("127.0.0.1", control_port), "renderer.counters"
                        ),
                        "chunk_view": _control_request(
                            ("127.0.0.1", control_port), "world.chunk_view"
                        ),
                        "local_player": _control_request(
                            ("127.0.0.1", control_port),
                            "world.client_local_player",
                        ),
                    }
                )
                result["screenshots"].append(
                    _take_weston_screenshot(
                        artifact_dir=artifact_dir,
                        environment=environment,
                        client="bbb",
                        segment=anchor["capture_segment"],
                        tick=anchor["server_tick"],
                    )
                )
            final_frame = replay["frames"][-1]
            _wait_for_log(
                server_log_path,
                (
                    f"BBB_BENCHMARK_MARKER {digest} "
                    f"{final_frame['capture_segment']} "
                    f"{final_frame['marker']} {final_frame['server_tick']}"
                ),
                process=client_process,
                timeout=timeout,
            )
            if mangohud_root is not None:
                result["mangohud_capture_stop"] = mangohud_set_logging(
                    process_pid=client_process.pid,
                    enabled=False,
                    completion_dir=artifact_dir / "mangohud",
                )
            else:
                result["app_quit_response"] = _control_request(
                    ("127.0.0.1", control_port), "app.quit"
                )
                try:
                    client_process.wait(timeout=20.0)
                except subprocess.TimeoutExpired:
                    result.setdefault("invalid_reasons", []).append(
                        "bbb did not exit after the ordered app.quit request"
                    )
            result["status"] = "passed"
    except BaseException as error:
        result["status"] = (
            "interrupted" if isinstance(error, KeyboardInterrupt) else "failed"
        )
        result["failure"] = str(error)
        raise
    finally:
        if client_group is not None:
            if mangohud_root is not None:
                result["client_cleanup"] = {
                    "signal": "SIGKILL",
                    "reason": (
                        "MangoHud 0.8.4 Vulkan on native Wayland reproducibly "
                        "crashes during ordered application teardown; "
                        "the control socket has already acknowledged logging stop"
                    ),
                }
                client_group.kill(timeout=10.0)
            else:
                result["client_cleanup"] = {"method": "app.quit"}
                client_group.cleanup(timeout=10.0)
            if client_group.process is not None:
                result["client_exit_status"] = client_group.process.returncode
        if server_group is not None:
            process = server_group.process
            if process is not None and process.poll() is None:
                try:
                    _write_server_commands(process, ("stop",))
                    process.wait(timeout=15.0)
                except (BenchmarkFailure, subprocess.TimeoutExpired):
                    pass
            server_group.cleanup(timeout=10.0)
        if compositor_group is not None:
            compositor_group.cleanup(timeout=10.0)
        if compositor_output is not None:
            compositor_output.close()
        if server_log_path.is_file():
            log = server_log_path.read_text(encoding="utf-8", errors="replace")
            result["marker_lines"] = [
                line for line in log.splitlines() if "BBB_BENCHMARK_" in line
            ]
            result["position_corrections"] = log.count(
                "moved too quickly"
            ) + log.count("moved wrongly")
            try:
                result["server_camera_log"] = write_server_camera_log(
                    artifact_dir / "server-camera.jsonl", log, replay
                )
            except BenchmarkFailure as error:
                result.setdefault("invalid_reasons", []).append(str(error))
        mango_dir = artifact_dir / "mangohud"
        if mango_dir.is_dir():
            result["mangohud_artifacts"] = [
                {
                    "path": str(path),
                    "size": path.stat().st_size,
                    "sha256": file_digest(path, "sha256"),
                }
                for path in sorted(mango_dir.rglob("*"))
                if path.is_file()
            ]
            raw_logs = sorted(
                path
                for path in mango_dir.rglob("*.csv")
                if not path.name.endswith("_summary.csv")
            )
            result["mangohud_raw_logs"] = [str(path) for path in raw_logs]
            client_log = (
                client_log_path.read_text(encoding="utf-8", errors="replace")
                if client_log_path.is_file()
                else ""
            )
            result["mangohud_hook_initialized"] = "[MANGOHUD]" in client_log
            if not result["mangohud_hook_initialized"]:
                result.setdefault("invalid_reasons", []).append(
                    "MangoHud Vulkan hook did not report initialization"
                )
            if not raw_logs:
                result.setdefault("invalid_reasons", []).append(
                    "MangoHud produced no raw per-frame CSV"
                )
            else:
                try:
                    result["mangohud_frame_summaries"] = [
                        asdict(summarize_frames(parse_mangohud_csv(path)))
                        for path in raw_logs
                    ]
                except BenchmarkFailure as error:
                    result.setdefault("invalid_reasons", []).append(str(error))
        if result.get("invalid_reasons") and result["status"] == "passed":
            result["status"] = "invalid"
        evidence_path.write_bytes(canonical_json_bytes(result))
    return result


def _json_ready(value: Any) -> Any:
    if hasattr(value, "__dataclass_fields__"):
        return asdict(value)
    return value


def _command_audit(args: argparse.Namespace) -> None:
    audit = audit_distribution(
        Path(args.mc_code_root),
        java_path=Path(args.java),
        verify_asset_objects=not args.skip_asset_objects,
    )
    print(json.dumps(_json_ready(audit), indent=2, sort_keys=True))


def _command_prepare_vanilla(args: argparse.Namespace) -> None:
    mc_root = Path(args.mc_code_root)
    artifact_dir = Path(args.artifact_dir)
    artifact_dir.mkdir(parents=True, exist_ok=True)
    audit = audit_distribution(mc_root, java_path=Path(args.java))
    natives = safe_extract_natives(
        (Path(entry.path) for entry in audit.native_classifiers),
        artifact_dir / "natives",
    )
    game_dir = artifact_dir / "game"
    options_path = write_vanilla_options(game_dir)
    plan = build_vanilla_launch_plan(
        audit,
        java_path=Path(args.java),
        game_dir=game_dir,
        assets_dir=mc_root / "downloads" / MINECRAFT_VERSION / "assets",
        natives_dir=artifact_dir / "natives",
        server_port=args.server_port,
    )
    output = {
        "schema": BENCHMARK_SCHEMA,
        "audit": asdict(audit),
        "natives": [asdict(entry) for entry in natives],
        "options": str(options_path),
        "options_hash": file_digest(options_path, "sha256"),
        "launch": {**asdict(plan), "argv": sanitized_argv(plan.argv)},
    }
    output_path = artifact_dir / "vanilla-launch.json"
    output_path.write_bytes(canonical_json_bytes(output))
    print(output_path)


def _command_replay_hash(args: argparse.Namespace) -> None:
    replay = load_json_object(Path(args.replay), "replay")
    print(replay_hash(replay))


def _command_write_default_replay(args: argparse.Namespace) -> None:
    replay = default_replay()
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(replay, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"{replay_hash(replay)}  {output}")


def _command_generate_datapack(args: argparse.Namespace) -> None:
    replay = load_json_object(Path(args.replay), "replay")
    print(generate_replay_datapack(replay, Path(args.output)))


def _command_prepare_world(args: argparse.Namespace) -> None:
    replay = load_json_object(Path(args.replay), "replay")
    snapshot = prepare_world_snapshot(
        replay=replay,
        mc_root=Path(args.mc_code_root),
        java_path=Path(args.java),
        artifact_dir=Path(args.artifact_dir),
        server_port=args.server_port or reserve_loopback_port(),
        accept_eula=args.accept_eula,
        timeout=args.timeout,
    )
    print(Path(args.artifact_dir) / "world-snapshot.json")
    print(snapshot.world_hash)


def _command_run_official_smoke(args: argparse.Namespace) -> None:
    replay = load_json_object(Path(args.replay), "replay")
    result = run_official_smoke(
        replay=replay,
        mc_root=Path(args.mc_code_root),
        java_path=Path(args.java),
        world_manifest_path=Path(args.world_manifest),
        artifact_dir=Path(args.artifact_dir),
        server_port=args.server_port or reserve_loopback_port(),
        accept_eula=args.accept_eula,
        timeout=args.timeout,
        wayland_display=args.wayland_display,
        mangohud_root=Path(args.mangohud_root) if args.mangohud_root else None,
        weston_path=Path(args.weston) if args.weston else None,
    )
    print(json.dumps(result, indent=2, sort_keys=True))


def _command_run_bbb_smoke(args: argparse.Namespace) -> None:
    replay = load_json_object(Path(args.replay), "replay")
    server_port = args.server_port or reserve_loopback_port()
    control_port = args.control_port or reserve_loopback_port()
    while control_port == server_port:
        control_port = reserve_loopback_port()
    result = run_bbb_smoke(
        replay=replay,
        mc_root=Path(args.mc_code_root),
        java_path=Path(args.java),
        bbb_binary=Path(args.bbb_binary),
        world_manifest_path=Path(args.world_manifest),
        artifact_dir=Path(args.artifact_dir),
        server_port=server_port,
        control_port=control_port,
        accept_eula=args.accept_eula,
        timeout=args.timeout,
        wayland_display=args.wayland_display,
        mangohud_root=Path(args.mangohud_root) if args.mangohud_root else None,
        weston_path=Path(args.weston),
    )
    print(json.dumps(result, indent=2, sort_keys=True))


def _command_summarize_mangohud(args: argparse.Namespace) -> None:
    capture = parse_mangohud_csv(Path(args.input))
    print(json.dumps(asdict(summarize_frames(capture)), indent=2, sort_keys=True))


def _command_capture_preflight(args: argparse.Namespace) -> None:
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = capture_preflight(
        java_path=Path(args.java),
        mangohud_root=Path(args.mangohud_root) if args.mangohud_root else None,
    )
    output.write_bytes(canonical_json_bytes(evidence))
    print(output)


def _command_summarize_blocked(args: argparse.Namespace) -> None:
    preflight_path = Path(args.preflight)
    vanilla_path = Path(args.vanilla_smoke)
    bbb_path = Path(args.bbb_smoke)
    preflight = load_json_object(preflight_path, "capture preflight")
    vanilla = load_json_object(vanilla_path, "vanilla smoke")
    bbb = load_json_object(bbb_path, "bbb smoke")
    preflight["artifact_path"] = str(preflight_path)
    vanilla["artifact_path"] = str(vanilla_path)
    bbb["artifact_path"] = str(bbb_path)
    summary = summarize_blocked_comparison(
        preflight=preflight,
        vanilla=vanilla,
        bbb=bbb,
        visible_content_status=args.visible_content_status,
        visible_content_note=args.visible_content_note,
    )
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json_bytes(summary))
    print(output)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    audit = subparsers.add_parser("audit")
    audit.add_argument("--mc-code-root", required=True)
    audit.add_argument("--java", required=True)
    audit.add_argument("--skip-asset-objects", action="store_true")
    audit.set_defaults(handler=_command_audit)

    prepare = subparsers.add_parser("prepare-vanilla")
    prepare.add_argument("--mc-code-root", required=True)
    prepare.add_argument("--java", required=True)
    prepare.add_argument("--artifact-dir", required=True)
    prepare.add_argument("--server-port", required=True, type=int)
    prepare.set_defaults(handler=_command_prepare_vanilla)

    replay = subparsers.add_parser("replay-hash")
    replay.add_argument("--replay", required=True)
    replay.set_defaults(handler=_command_replay_hash)

    default_replay_parser = subparsers.add_parser("write-default-replay")
    default_replay_parser.add_argument("--output", required=True)
    default_replay_parser.set_defaults(handler=_command_write_default_replay)

    datapack = subparsers.add_parser("generate-datapack")
    datapack.add_argument("--replay", required=True)
    datapack.add_argument("--output", required=True)
    datapack.set_defaults(handler=_command_generate_datapack)

    prepare_world = subparsers.add_parser("prepare-world")
    prepare_world.add_argument("--replay", required=True)
    prepare_world.add_argument("--mc-code-root", required=True)
    prepare_world.add_argument("--java", required=True)
    prepare_world.add_argument("--artifact-dir", required=True)
    prepare_world.add_argument("--server-port", type=int)
    prepare_world.add_argument("--timeout", type=float, default=300.0)
    prepare_world.add_argument("--accept-eula", action="store_true")
    prepare_world.set_defaults(handler=_command_prepare_world)

    smoke = subparsers.add_parser("run-official-smoke")
    smoke.add_argument("--replay", required=True)
    smoke.add_argument("--mc-code-root", required=True)
    smoke.add_argument("--java", required=True)
    smoke.add_argument("--world-manifest", required=True)
    smoke.add_argument("--artifact-dir", required=True)
    smoke.add_argument("--server-port", type=int)
    smoke.add_argument("--timeout", type=float, default=240.0)
    smoke.add_argument("--accept-eula", action="store_true")
    smoke.add_argument("--wayland-display", default="bbb-gate")
    smoke.add_argument("--mangohud-root")
    smoke.add_argument("--weston")
    smoke.set_defaults(handler=_command_run_official_smoke)

    bbb_smoke = subparsers.add_parser("run-bbb-smoke")
    bbb_smoke.add_argument("--replay", required=True)
    bbb_smoke.add_argument("--mc-code-root", required=True)
    bbb_smoke.add_argument("--java", required=True)
    bbb_smoke.add_argument("--bbb-binary", required=True)
    bbb_smoke.add_argument("--world-manifest", required=True)
    bbb_smoke.add_argument("--artifact-dir", required=True)
    bbb_smoke.add_argument("--server-port", type=int)
    bbb_smoke.add_argument("--control-port", type=int)
    bbb_smoke.add_argument("--timeout", type=float, default=240.0)
    bbb_smoke.add_argument("--accept-eula", action="store_true")
    bbb_smoke.add_argument("--wayland-display", default="bbb-benchmark-native")
    bbb_smoke.add_argument("--mangohud-root")
    bbb_smoke.add_argument("--weston", required=True)
    bbb_smoke.set_defaults(handler=_command_run_bbb_smoke)

    mango = subparsers.add_parser("summarize-mangohud")
    mango.add_argument("--input", required=True)
    mango.set_defaults(handler=_command_summarize_mangohud)

    preflight = subparsers.add_parser("capture-preflight")
    preflight.add_argument("--java", required=True)
    preflight.add_argument("--mangohud-root")
    preflight.add_argument("--output", required=True)
    preflight.set_defaults(handler=_command_capture_preflight)

    blocked = subparsers.add_parser("summarize-blocked")
    blocked.add_argument("--preflight", required=True)
    blocked.add_argument("--vanilla-smoke", required=True)
    blocked.add_argument("--bbb-smoke", required=True)
    blocked.add_argument(
        "--visible-content-status",
        choices=("match", "mismatch", "unverified"),
        required=True,
    )
    blocked.add_argument("--visible-content-note", required=True)
    blocked.add_argument("--output", required=True)
    blocked.set_defaults(handler=_command_summarize_blocked)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        args.handler(args)
    except BenchmarkFailure as error:
        print(f"official client benchmark failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
