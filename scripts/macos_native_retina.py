#!/usr/bin/env python3
"""macOS built-in-display native-drawable performance acceptance.

This is deliberately separate from ``official_client_benchmark.py``'s fixed
1280x720 Linux baseline.  CoreGraphics supplies the current backing-pixel
target, screenshots attest the drawable actually rendered by each client, and
formal aggregation refuses to compare runs when those pixels differ.
"""

from __future__ import annotations

import argparse
import csv
import ctypes
import hashlib
import json
import math
import os
import platform
import re
import shutil
import statistics
import struct
import subprocess
import sys
import time
import zipfile
from collections.abc import Mapping, Sequence
from dataclasses import asdict, dataclass
from pathlib import Path

from bbb_tools.input.contract import InputError
from bbb_tools.input.macos_quartz import MacOsQuartzBackend
from official_client_benchmark import (
    BENCHMARK_USERNAME,
    MINECRAFT_VERSION,
    SERVER_READY_MARKER,
    VIEW_DISTANCE,
    BenchmarkFailure,
    ProcessGroup,
    VerifiedFile,
    _native_classifier,
    _safe_relative_artifact_path,
    _validate_zip_member,
    _verify_world_snapshot,
    _wait_for_log,
    _warmup_and_start_replay,
    _write_server_commands,
    audit_distribution,
    audit_vanilla_options,
    benchmark_marker_line,
    build_bbb_launch_argv,
    canonical_json_bytes,
    canonical_sha256,
    file_digest,
    library_allowed,
    load_json_object,
    prepare_world_snapshot,
    replay_capture_markers,
    reserve_loopback_port,
    sanitized_argv,
    validate_file,
    validate_server_jar,
    vanilla_options,
    write_server_layout,
    write_vanilla_options,
)

SCHEMA = 1
PROFILE_ID = "macos-native-retina"
FORMAL_ORDER = ("vanilla", "bbb", "bbb", "vanilla") * 2
CAPTURE_SEGMENTS = (
    "static_orbit",
    "preloaded_flight",
    "chunk_boundary_crossing",
)
WARMUP_SECONDS = 30
PROFILE_SECONDS = 10
UNLIMITED_FPS_SENTINEL = 260
HITCH_THRESHOLDS_MS = (33.333, 50.0, 100.0)
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
F2_KEY_CODE = 120
F3_KEY_CODE = 99
L_KEY_CODE = 37
PROFILE_CONTRACT_PATH = (
    Path(__file__).resolve().parent
    / "fixtures"
    / "official-client-benchmark"
    / "macos-native-retina-26.1.json"
)
REPO_ROOT = Path(__file__).resolve().parents[1]


class RetinaAcceptanceError(RuntimeError):
    """The native-Retina contract or one of its evidence files is invalid."""


def load_profile_contract(path: Path = PROFILE_CONTRACT_PATH) -> dict[str, object]:
    try:
        contract = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RetinaAcceptanceError(f"cannot read profile contract {path}: {error}") from error
    expected = {
        "schema": SCHEMA,
        "profile": PROFILE_ID,
        "minecraft_version": MINECRAFT_VERSION,
        "window_mode": "fullscreen",
        "vsync": False,
        "framerate_limit": "unlimited",
        "framerate_limit_sentinel": UNLIMITED_FPS_SENTINEL,
        "warmup_seconds": WARMUP_SECONDS,
        "profile_seconds": PROFILE_SECONDS,
        "formal_order": list(FORMAL_ORDER),
        "capture_segments": list(CAPTURE_SEGMENTS),
        "drawable_mismatch_policy": "block-without-comparison",
    }
    if not isinstance(contract, dict) or any(
        contract.get(name) != value for name, value in expected.items()
    ):
        raise RetinaAcceptanceError("macos-native-retina profile contract is stale")
    return contract


def require_clean_full_gated_head(repo: Path = REPO_ROOT) -> str:
    head = subprocess.run(
        ("git", "rev-parse", "HEAD"),
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    status = subprocess.run(
        ("git", "status", "--porcelain"),
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    stamp = repo / ".git" / "bbb" / "last-gated"
    gated = stamp.read_text(encoding="utf-8").strip() if stamp.is_file() else ""
    if status:
        raise RetinaAcceptanceError("formal capture requires a clean worktree")
    if gated != head:
        raise RetinaAcceptanceError(
            f"formal capture requires full-gated HEAD: head={head}, last-gated={gated or '<none>'}"
        )
    return head


class CGPoint(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]


class CGSize(ctypes.Structure):
    _fields_ = [("width", ctypes.c_double), ("height", ctypes.c_double)]


class CGRect(ctypes.Structure):
    _fields_ = [("origin", CGPoint), ("size", CGSize)]


@dataclass(frozen=True)
class DisplayProbe:
    display_id: int
    name: str
    builtin: bool
    main: bool
    vendor_id: int | None
    product_id: int | None
    serial_number: int | None
    logical_width: int
    logical_height: int
    scale_factor: float
    backing_pixel_width: int
    backing_pixel_height: int
    panel_native_width: int | None
    panel_native_height: int | None
    refresh_hz: float | None
    metal_device: str

    @property
    def target_drawable(self) -> tuple[int, int]:
        return (self.backing_pixel_width, self.backing_pixel_height)


@dataclass(frozen=True)
class MacDistribution:
    libraries: tuple[VerifiedFile, ...]
    native_classifiers: tuple[VerifiedFile, ...]
    client: VerifiedFile
    main_class: str
    asset_index_id: str
    resource_hash: str


def _macos_native_classifier_matches(name: str, arch: str = "arm64") -> bool:
    if not _native_classifier(name):
        return True
    classifier = name.split(":")[3]
    if arch in {"arm64", "aarch64"}:
        return classifier == "natives-macos-arm64"
    return classifier == "natives-macos"


def audit_macos_distribution(vanilla_root: Path, *, java_path: Path) -> MacDistribution:
    """Select Mojang's arm64 macOS rows without changing the Linux baseline."""

    base = audit_distribution(vanilla_root, java_path=java_path)
    downloads = vanilla_root / "downloads" / MINECRAFT_VERSION
    metadata = load_json_object(downloads / f"{MINECRAFT_VERSION}.json", "version metadata")
    raw_libraries = metadata.get("libraries")
    if not isinstance(raw_libraries, list):
        raise RetinaAcceptanceError("version metadata lacks libraries")
    libraries: list[VerifiedFile] = []
    natives: list[VerifiedFile] = []
    for library in raw_libraries:
        if not isinstance(library, dict) or not library_allowed(
            library,
            os_name="osx",
            os_arch="arm64",
            os_version=platform.release(),
        ):
            continue
        name = library.get("name")
        artifact = library.get("downloads", {}).get("artifact")
        if not isinstance(name, str) or not isinstance(artifact, dict):
            raise RetinaAcceptanceError("allowed macOS library lacks an artifact")
        if not _macos_native_classifier_matches(name):
            continue
        relative = _safe_relative_artifact_path(artifact.get("path"), f"macOS library {name}")
        verified = validate_file(
            downloads / "libraries" / relative,
            expected_sha1=str(artifact.get("sha1", "")),
            expected_size=int(artifact.get("size", -1)),
            label=f"macOS library {name}",
        )
        (natives if _native_classifier(name) else libraries).append(verified)
    if not natives:
        raise RetinaAcceptanceError("version metadata selected no macOS arm64 natives")
    resource_hash = canonical_sha256(
        {
            "client": base.client.sha1,
            "asset_index": base.asset_index.sha1,
            "asset_objects": base.asset_objects,
            "asset_object_bytes": base.asset_object_bytes,
        }
    )
    return MacDistribution(
        libraries=tuple(libraries),
        native_classifiers=tuple(natives),
        client=base.client,
        main_class=base.main_class,
        asset_index_id=base.asset_index_id,
        resource_hash=resource_hash,
    )


def safe_extract_macos_natives(
    archives: Sequence[Path], destination: Path
) -> tuple[VerifiedFile, ...]:
    destination.mkdir(parents=True, exist_ok=True)
    extracted: dict[str, VerifiedFile] = {}
    for archive in archives:
        try:
            jar = zipfile.ZipFile(archive)
        except (OSError, zipfile.BadZipFile) as error:
            raise RetinaAcceptanceError(
                f"cannot open macOS native archive {archive}: {error}"
            ) from error
        with jar:
            for info in jar.infolist():
                member = _validate_zip_member(info, archive)
                if info.is_dir() or not member.name.endswith((".dylib", ".jnilib")):
                    continue
                payload = jar.read(info)
                digest = hashlib.sha1(payload).hexdigest()
                existing = extracted.get(member.name)
                if existing is not None:
                    if existing.sha1 != digest or existing.size != len(payload):
                        raise RetinaAcceptanceError(
                            f"macOS native basename collision for {member.name}"
                        )
                    continue
                output = destination / member.name
                if output.exists():
                    if (
                        not output.is_file()
                        or output.stat().st_size != len(payload)
                        or file_digest(output) != digest
                    ):
                        raise RetinaAcceptanceError(
                            f"refusing to replace macOS native target {output}"
                        )
                else:
                    temporary = output.with_name(output.name + ".part")
                    temporary.write_bytes(payload)
                    os.replace(temporary, output)
                    output.chmod(0o755)
                extracted[member.name] = VerifiedFile(str(output), digest, len(payload))
    if not extracted:
        raise RetinaAcceptanceError("macOS native classifiers contained no dylibs")
    return tuple(extracted[name] for name in sorted(extracted))


def _parse_pair(value: object) -> tuple[int, int] | None:
    if not isinstance(value, str):
        return None
    import re

    match = re.search(r"([0-9]+)\s*x\s*([0-9]+)", value)
    if match is None:
        return None
    return (int(match.group(1)), int(match.group(2)))


def normalize_display_probe(
    coregraphics: Mapping[str, object], profiler: Mapping[str, object]
) -> DisplayProbe:
    """Join CoreGraphics current-mode facts to one system_profiler display."""

    display_id = int(coregraphics.get("display_id", 0))
    logical = coregraphics.get("logical_size")
    backing = coregraphics.get("backing_pixel_size")
    if (
        not isinstance(logical, Sequence)
        or isinstance(logical, (str, bytes))
        or len(logical) != 2
        or not isinstance(backing, Sequence)
        or isinstance(backing, (str, bytes))
        or len(backing) != 2
    ):
        raise RetinaAcceptanceError("CoreGraphics display dimensions are malformed")
    logical_width, logical_height = (int(logical[0]), int(logical[1]))
    pixel_width, pixel_height = (int(backing[0]), int(backing[1]))
    if min(display_id, logical_width, logical_height, pixel_width, pixel_height) <= 0:
        raise RetinaAcceptanceError("CoreGraphics returned a zero display dimension")
    scale_x = pixel_width / logical_width
    scale_y = pixel_height / logical_height
    if not math.isclose(scale_x, scale_y, rel_tol=0.0, abs_tol=1e-6):
        raise RetinaAcceptanceError(
            f"non-uniform display scale: horizontal={scale_x}, vertical={scale_y}"
        )
    profiler_id = profiler.get("_spdisplays_displayID")
    if profiler_id is not None and int(str(profiler_id), 0) != display_id:
        raise RetinaAcceptanceError(
            f"system_profiler display id {profiler_id} differs from CoreGraphics {display_id}"
        )
    panel_native = _parse_pair(profiler.get("spdisplays_pixelresolution"))

    def optional_hex(name: str) -> int | None:
        value = profiler.get(name)
        if value is None:
            return None
        try:
            return int(str(value), 16)
        except ValueError as error:
            raise RetinaAcceptanceError(f"invalid system_profiler {name}: {value!r}") from error

    refresh = coregraphics.get("refresh_hz")
    refresh_hz = None if refresh in (None, 0, 0.0) else float(refresh)
    return DisplayProbe(
        display_id=display_id,
        name=str(profiler.get("_name", "built-in display")),
        builtin=bool(coregraphics.get("builtin")),
        main=bool(coregraphics.get("main")),
        vendor_id=optional_hex("_spdisplays_display-vendor-id"),
        product_id=optional_hex("_spdisplays_display-product-id"),
        serial_number=optional_hex("_spdisplays_display-serial-number"),
        logical_width=logical_width,
        logical_height=logical_height,
        scale_factor=scale_x,
        backing_pixel_width=pixel_width,
        backing_pixel_height=pixel_height,
        panel_native_width=None if panel_native is None else panel_native[0],
        panel_native_height=None if panel_native is None else panel_native[1],
        refresh_hz=refresh_hz,
        metal_device=str(coregraphics.get("metal_device", "unknown")),
    )


def _coregraphics_displays() -> list[dict[str, object]]:
    if sys.platform != "darwin":
        raise RetinaAcceptanceError("display probing is available only on macOS")
    api = ctypes.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
    api.CGGetOnlineDisplayList.argtypes = [
        ctypes.c_uint32,
        ctypes.POINTER(ctypes.c_uint32),
        ctypes.POINTER(ctypes.c_uint32),
    ]
    api.CGGetOnlineDisplayList.restype = ctypes.c_int32
    api.CGDisplayIsBuiltin.argtypes = [ctypes.c_uint32]
    api.CGDisplayIsBuiltin.restype = ctypes.c_uint32
    api.CGDisplayIsMain.argtypes = [ctypes.c_uint32]
    api.CGDisplayIsMain.restype = ctypes.c_uint32
    api.CGDisplayBounds.argtypes = [ctypes.c_uint32]
    api.CGDisplayBounds.restype = CGRect
    api.CGDisplayCopyDisplayMode.argtypes = [ctypes.c_uint32]
    api.CGDisplayCopyDisplayMode.restype = ctypes.c_void_p
    api.CGDisplayModeGetPixelWidth.argtypes = [ctypes.c_void_p]
    api.CGDisplayModeGetPixelWidth.restype = ctypes.c_size_t
    api.CGDisplayModeGetPixelHeight.argtypes = [ctypes.c_void_p]
    api.CGDisplayModeGetPixelHeight.restype = ctypes.c_size_t
    api.CGDisplayModeGetRefreshRate.argtypes = [ctypes.c_void_p]
    api.CGDisplayModeGetRefreshRate.restype = ctypes.c_double
    core_foundation = ctypes.CDLL(
        "/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation"
    )
    core_foundation.CFRelease.argtypes = [ctypes.c_void_p]

    count = ctypes.c_uint32()
    status = api.CGGetOnlineDisplayList(0, None, ctypes.byref(count))
    if status != 0 or count.value == 0:
        raise RetinaAcceptanceError(f"CoreGraphics could not enumerate online displays ({status})")
    identifiers = (ctypes.c_uint32 * count.value)()
    status = api.CGGetOnlineDisplayList(count, identifiers, ctypes.byref(count))
    if status != 0:
        raise RetinaAcceptanceError(f"CoreGraphics could not read online display ids ({status})")
    values: list[dict[str, object]] = []
    for display_id in identifiers[: count.value]:
        mode = api.CGDisplayCopyDisplayMode(display_id)
        if not mode:
            raise RetinaAcceptanceError(f"CoreGraphics display {display_id} has no current mode")
        try:
            bounds = api.CGDisplayBounds(display_id)
            values.append(
                {
                    "display_id": int(display_id),
                    "builtin": bool(api.CGDisplayIsBuiltin(display_id)),
                    "main": bool(api.CGDisplayIsMain(display_id)),
                    "logical_size": [
                        round(bounds.size.width),
                        round(bounds.size.height),
                    ],
                    "backing_pixel_size": [
                        int(api.CGDisplayModeGetPixelWidth(mode)),
                        int(api.CGDisplayModeGetPixelHeight(mode)),
                    ],
                    "refresh_hz": float(api.CGDisplayModeGetRefreshRate(mode)),
                }
            )
        finally:
            core_foundation.CFRelease(mode)
    return values


def _system_profiler_displays() -> tuple[str, list[dict[str, object]]]:
    result = subprocess.run(
        ("system_profiler", "SPDisplaysDataType", "-json", "-detailLevel", "mini"),
        check=False,
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        timeout=30.0,
    )
    if result.returncode != 0:
        raise RetinaAcceptanceError(
            "system_profiler display query failed: " + result.stderr.strip()
        )
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RetinaAcceptanceError("system_profiler returned invalid JSON") from error
    adapters = payload.get("SPDisplaysDataType")
    if not isinstance(adapters, list):
        raise RetinaAcceptanceError("system_profiler lacks SPDisplaysDataType")
    displays: list[dict[str, object]] = []
    device = "unknown"
    for adapter in adapters:
        if not isinstance(adapter, dict):
            continue
        adapter_device = str(adapter.get("sppci_model", adapter.get("_name", "unknown")))
        rows = adapter.get("spdisplays_ndrvs", [])
        if not isinstance(rows, list):
            continue
        for row in rows:
            if isinstance(row, dict):
                displays.append(row)
                if row.get("spdisplays_connection_type") == "spdisplays_internal":
                    device = adapter_device
    return device, displays


def probe_builtin_display() -> DisplayProbe:
    coregraphics = [row for row in _coregraphics_displays() if row["builtin"]]
    if len(coregraphics) != 1:
        raise RetinaAcceptanceError(
            f"expected exactly one online built-in display, observed {len(coregraphics)}"
        )
    metal_device, profiler_displays = _system_profiler_displays()
    display_id = int(coregraphics[0]["display_id"])
    matches = [
        row
        for row in profiler_displays
        if str(row.get("_spdisplays_displayID", "")) == str(display_id)
    ]
    if len(matches) != 1:
        raise RetinaAcceptanceError(
            f"system_profiler did not identify built-in display {display_id} exactly once"
        )
    joined = dict(coregraphics[0])
    joined["metal_device"] = metal_device
    return normalize_display_probe(joined, matches[0])


def launch_contract(
    display: DisplayProbe, *, server_port: int, control_port: int
) -> dict[str, object]:
    if not 1 <= server_port <= 65535 or not 1 <= control_port <= 65535:
        raise RetinaAcceptanceError("launch ports must be in 1..65535")
    drawable = [display.backing_pixel_width, display.backing_pixel_height]
    logical = [display.logical_width, display.logical_height]
    common = {
        "profile": PROFILE_ID,
        "minecraft_version": MINECRAFT_VERSION,
        "username": BENCHMARK_USERNAME,
        "window_mode": "fullscreen",
        "logical_size": logical,
        "target_drawable_pixels": drawable,
        "view_distance_chunks": VIEW_DISTANCE,
        "resource_packs": ["vanilla"],
        "vsync": False,
        "framerate_limit": "unlimited",
        "framerate_limit_sentinel": UNLIMITED_FPS_SENTINEL,
        "warmup_seconds": WARMUP_SECONDS,
        "profile_seconds": PROFILE_SECONDS,
        "capture_segments": list(CAPTURE_SEGMENTS),
        "input_script": [
            "focus exact client pid",
            "at each capture_start: F2 screenshot, then hold F3 and tap L",
        ],
    }
    official = (
        "--width",
        str(display.logical_width),
        "--height",
        str(display.logical_height),
        "--fullscreen",
        "--quickPlayMultiplayer",
        f"127.0.0.1:{server_port}",
    )
    bbb = (
        "--width",
        str(display.backing_pixel_width),
        "--height",
        str(display.backing_pixel_height),
        "--client-fullscreen",
        "true",
        "--client-vsync",
        "false",
        "--client-framerate-limit",
        str(UNLIMITED_FPS_SENTINEL),
        "--control",
        f"127.0.0.1:{control_port}",
    )
    return {"common": common, "official_arguments": list(official), "bbb_arguments": list(bbb)}


def png_dimensions(path: Path) -> tuple[int, int]:
    try:
        header = path.read_bytes()[:24]
    except OSError as error:
        raise RetinaAcceptanceError(f"cannot read screenshot {path}: {error}") from error
    if len(header) != 24 or not header.startswith(PNG_SIGNATURE) or header[12:16] != b"IHDR":
        raise RetinaAcceptanceError(f"screenshot is not a valid PNG header: {path}")
    width, height = struct.unpack(">II", header[16:24])
    if width == 0 or height == 0:
        raise RetinaAcceptanceError(f"screenshot has a zero dimension: {path}")
    return width, height


def _read_metric_csv(archive: zipfile.ZipFile, suffix: str, column: str) -> dict[int, float]:
    names = [name for name in archive.namelist() if name.endswith(suffix)]
    if len(names) != 1:
        raise RetinaAcceptanceError(
            f"profiling archive must contain one {suffix}, observed {len(names)}"
        )
    text = archive.read(names[0]).decode("utf-8")
    rows: dict[int, float] = {}
    for row in csv.DictReader(text.splitlines()):
        try:
            tick = int(row["@tick"])
            value = float(row[column])
        except (KeyError, TypeError, ValueError) as error:
            raise RetinaAcceptanceError(
                f"profiling metric {column} contains malformed data"
            ) from error
        if not math.isfinite(value):
            raise RetinaAcceptanceError(f"profiling metric {column} is non-finite")
        rows[tick] = value
    if not rows:
        raise RetinaAcceptanceError(f"profiling metric {column} is empty")
    return rows


def percentile(values: Sequence[float], percent: int) -> float:
    if not values or not 0 < percent <= 100:
        raise RetinaAcceptanceError("percentile requires samples and a rank in 1..100")
    ordered = sorted(values)
    rank = math.ceil(len(ordered) * percent / 100)
    return ordered[max(0, rank - 1)]


def summarize_durations_ms(values: Sequence[float]) -> dict[str, object]:
    if not values or any(not math.isfinite(value) or value <= 0 for value in values):
        raise RetinaAcceptanceError("duration samples must be finite and positive")
    ordered = sorted(values)
    slow_count = max(1, math.ceil(len(ordered) * 0.01))
    slow_mean = statistics.fmean(ordered[-slow_count:])
    mean_ms = statistics.fmean(ordered)
    return {
        "samples": len(values),
        "mean_ms": mean_ms,
        "p50_ms": percentile(ordered, 50),
        "p95_ms": percentile(ordered, 95),
        "p99_ms": percentile(ordered, 99),
        "one_percent_low_fps": 1000.0 / slow_mean,
        "mean_fps": 1000.0 / mean_ms,
        "coefficient_of_variation": (
            statistics.pstdev(ordered) / mean_ms if len(ordered) > 1 else 0.0
        ),
        "hitch_counts": {
            f">={threshold:.3f}ms": sum(value >= threshold for value in ordered)
            for threshold in HITCH_THRESHOLDS_MS
        },
    }


def profiler_samples(path: Path) -> dict[str, list[float]]:
    try:
        archive = zipfile.ZipFile(path)
    except (OSError, zipfile.BadZipFile) as error:
        raise RetinaAcceptanceError(f"cannot open profiling archive {path}: {error}") from error
    with archive:
        ticking = _read_metric_csv(archive, "/metrics/ticking.csv", "ticktime")
        gpu = _read_metric_csv(archive, "/metrics/gpu.csv", "gpuUtilization")
    common_ticks = sorted(ticking.keys() & gpu.keys())
    if not common_ticks:
        raise RetinaAcceptanceError("profiling ticktime and GPU samples do not overlap")
    frame_ms = [ticking[tick] / 1_000_000.0 for tick in common_ticks]
    gpu_ms = [
        ticking[tick] * gpu[tick] / 100.0 / 1_000_000.0 for tick in common_ticks if gpu[tick] > 0.0
    ]
    if not gpu_ms:
        raise RetinaAcceptanceError("profiling archive contains no positive GPU timer sample")
    return {
        "application_frame_time": frame_ms,
        "gpu_duration": gpu_ms,
    }


def parse_profiler_archive(path: Path) -> dict[str, object]:
    samples = profiler_samples(path)
    return {
        "path": str(path),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "application_frame_time": summarize_durations_ms(samples["application_frame_time"]),
        "gpu_duration": summarize_durations_ms(samples["gpu_duration"]),
        "gpu_duration_scope": (
            "render-API timer-query duration reconstructed as "
            "ticktime * gpuUtilization / 100; both clients use the unmodified "
            "26.1 F3+L metric names"
        ),
    }


def settings_identity() -> dict[str, object]:
    values = vanilla_options()
    selected = {
        name: values[name]
        for name in (
            "enableVsync",
            "maxFps",
            "renderDistance",
            "simulationDistance",
            "graphicsPreset",
            "ao",
            "renderClouds",
            "particles",
            "mipmapLevels",
            "cutoutLeaves",
            "improvedTransparency",
            "textureFiltering",
            "maxAnisotropyBit",
            "fov",
            "gamma",
            "resourcePacks",
        )
    }
    return {
        "values": selected,
        "sha256": hashlib.sha256(canonical_json_bytes(selected)).hexdigest(),
    }


def _load_object(path: Path, label: str) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RetinaAcceptanceError(f"cannot read {label} {path}: {error}") from error
    if not isinstance(value, dict):
        raise RetinaAcceptanceError(f"{label} root must be an object: {path}")
    return value


def _combined_summary(
    runs: Sequence[Mapping[str, object]], client: str, metric: str
) -> dict[str, object]:
    # Formal raw profile archives remain authoritative. Reparse them here so a
    # hand-edited run summary cannot influence the final comparison.
    values: list[float] = []
    for run in runs:
        if run.get("client") != client:
            continue
        profiles = run.get("profiles")
        if not isinstance(profiles, list):
            raise RetinaAcceptanceError(f"{client} run lacks profiler archives")
        for profile in profiles:
            if not isinstance(profile, Mapping) or not isinstance(profile.get("path"), str):
                raise RetinaAcceptanceError(f"{client} profiler entry is malformed")
            parsed = parse_profiler_archive(Path(str(profile["path"])))
            # Store aggregate summaries by weighting their sample population is
            # insufficient for percentiles, so reports also carry sample files.
            # The runner writes those exact raw values next to each archive.
            samples_path = profile.get(f"{metric}_samples")
            if not isinstance(samples_path, str):
                raise RetinaAcceptanceError(f"{client} profiler entry lacks {metric} raw samples")
            raw = _load_object(Path(samples_path), f"{metric} samples")
            samples = raw.get("milliseconds")
            if not isinstance(samples, list):
                raise RetinaAcceptanceError(f"{metric} raw samples are malformed")
            values.extend(float(value) for value in samples)
            if parsed[metric]["samples"] != len(samples):
                raise RetinaAcceptanceError(f"{metric} raw sample count differs from archive")
    return summarize_durations_ms(values)


def _combined_process_cpu_summary(
    runs: Sequence[Mapping[str, object]], client: str
) -> dict[str, object]:
    cpu_ms = 0.0
    frames = 0
    for run in runs:
        if run.get("client") != client:
            continue
        profiles = run.get("profiles")
        if not isinstance(profiles, list):
            raise RetinaAcceptanceError(f"{client} run lacks CPU profile windows")
        for profile in profiles:
            if not isinstance(profile, Mapping):
                raise RetinaAcceptanceError(f"{client} CPU profile entry is malformed")
            cpu = profile.get("process_cpu_frame_time")
            if not isinstance(cpu, Mapping):
                raise RetinaAcceptanceError(f"{client} CPU profile entry lacks rusage")
            cpu_ms += float(profile["process_cpu_time_ms"])
            frames += int(cpu["frames"])
    if cpu_ms <= 0.0 or frames <= 0:
        raise RetinaAcceptanceError(f"{client} process CPU samples are empty")
    return {
        "frames": frames,
        "total_cpu_ms": cpu_ms,
        "mean_ms": cpu_ms / frames,
        "scope": (
            "proc_pid_rusage user+system CPU time across all F3+L windows, "
            "normalized by sampled application frames; no CPU-time percentile claim"
        ),
    }


def validate_and_summarize(
    display: DisplayProbe, runs: Sequence[Mapping[str, object]]
) -> dict[str, object]:
    """Return a formal comparison or a blocker without comparison metrics."""

    base = {
        "schema": SCHEMA,
        "profile": PROFILE_ID,
        "display": asdict(display),
        "target_drawable_pixels": list(display.target_drawable),
        "formal_order": list(FORMAL_ORDER),
        "window_mode": "fullscreen",
        "vsync": False,
        "framerate_limit": "unlimited",
        "warmup_seconds": WARMUP_SECONDS,
        "runs": [dict(run) for run in runs],
    }
    reasons: list[dict[str, object]] = []
    clients = tuple(str(run.get("client", "")) for run in runs)
    if clients != FORMAL_ORDER:
        reasons.append(
            {
                "code": "formal_order_mismatch",
                "expected": list(FORMAL_ORDER),
                "actual": list(clients),
            }
        )
    expected_drawable = display.target_drawable
    identity_fields = (
        "replay_sha256",
        "world_sha256",
        "settings_sha256",
        "resource_pack_sha256",
        "input_script_sha256",
    )
    for field in identity_fields:
        values = {str(run.get(field, "")) for run in runs}
        if len(values) != 1 or "" in values:
            reasons.append(
                {"code": "contract_identity_mismatch", "field": field, "values": sorted(values)}
            )
    for index, run in enumerate(runs, start=1):
        if run.get("status") != "passed":
            reasons.append({"code": "run_not_passed", "run": index, "status": run.get("status")})
        if run.get("vsync") is not False or run.get("framerate_limit") != "unlimited":
            reasons.append({"code": "frame_pacing_contract_mismatch", "run": index})
        if float(run.get("warmup_seconds", -1)) < WARMUP_SECONDS:
            reasons.append({"code": "insufficient_warmup", "run": index})
        observed = run.get("drawable_pixels")
        if (
            not isinstance(observed, Sequence)
            or isinstance(observed, (str, bytes))
            or len(observed) != 2
        ):
            actual: object = observed
        else:
            actual = [int(observed[0]), int(observed[1])]
        if actual != list(expected_drawable):
            reasons.append(
                {
                    "code": "drawable_mismatch",
                    "run": index,
                    "client": run.get("client"),
                    "expected": list(expected_drawable),
                    "actual": actual,
                }
            )
        screenshots = run.get("screenshots")
        segments = (
            {
                str(item.get("segment")): item
                for item in screenshots
                if isinstance(screenshots, list) and isinstance(item, Mapping)
            }
            if isinstance(screenshots, list)
            else {}
        )
        if set(segments) != set(CAPTURE_SEGMENTS):
            reasons.append(
                {"code": "screenshot_segments_mismatch", "run": index, "segments": sorted(segments)}
            )
        for segment, item in segments.items():
            size = item.get("pixels")
            if size != list(expected_drawable):
                reasons.append(
                    {
                        "code": "drawable_mismatch",
                        "run": index,
                        "client": run.get("client"),
                        "segment": segment,
                        "expected": list(expected_drawable),
                        "actual": size,
                    }
                )
        profiles = run.get("profiles")
        if not isinstance(profiles, list) or len(profiles) != len(CAPTURE_SEGMENTS):
            reasons.append({"code": "profile_segments_mismatch", "run": index})
    if reasons:
        return {
            **base,
            "status": "blocked",
            "formal_comparison_complete": False,
            "metrics_publishable": False,
            "blockers": reasons,
            "comparison": None,
        }
    comparison = {
        client: {
            "application_frame_time": _combined_summary(runs, client, "application_frame_time"),
            "process_cpu_frame_time": _combined_process_cpu_summary(runs, client),
            "gpu_duration": _combined_summary(runs, client, "gpu_duration"),
        }
        for client in ("vanilla", "bbb")
    }
    return {
        **base,
        "status": "passed",
        "formal_comparison_complete": True,
        "metrics_publishable": True,
        "blockers": [],
        "comparison": comparison,
    }


class RUsageInfoV2(ctypes.Structure):
    _fields_ = [
        ("ri_uuid", ctypes.c_uint8 * 16),
        ("ri_user_time", ctypes.c_uint64),
        ("ri_system_time", ctypes.c_uint64),
        ("ri_pkg_idle_wkups", ctypes.c_uint64),
        ("ri_interrupt_wkups", ctypes.c_uint64),
        ("ri_pageins", ctypes.c_uint64),
        ("ri_wired_size", ctypes.c_uint64),
        ("ri_resident_size", ctypes.c_uint64),
        ("ri_phys_footprint", ctypes.c_uint64),
        ("ri_proc_start_abstime", ctypes.c_uint64),
        ("ri_proc_exit_abstime", ctypes.c_uint64),
        ("ri_child_user_time", ctypes.c_uint64),
        ("ri_child_system_time", ctypes.c_uint64),
        ("ri_child_pkg_idle_wkups", ctypes.c_uint64),
        ("ri_child_interrupt_wkups", ctypes.c_uint64),
        ("ri_child_pageins", ctypes.c_uint64),
        ("ri_child_elapsed_abstime", ctypes.c_uint64),
        ("ri_diskio_bytesread", ctypes.c_uint64),
        ("ri_diskio_byteswritten", ctypes.c_uint64),
    ]


class MachTimebaseInfo(ctypes.Structure):
    _fields_ = [("numer", ctypes.c_uint32), ("denom", ctypes.c_uint32)]


def process_cpu_time_nanos(pid: int) -> int:
    if sys.platform != "darwin" or pid <= 0:
        raise RetinaAcceptanceError("process CPU sampling requires a positive macOS pid")
    api = ctypes.CDLL("/usr/lib/libproc.dylib")
    api.proc_pid_rusage.argtypes = [
        ctypes.c_int,
        ctypes.c_int,
        ctypes.POINTER(RUsageInfoV2),
    ]
    api.proc_pid_rusage.restype = ctypes.c_int
    usage = RUsageInfoV2()
    status = api.proc_pid_rusage(pid, 2, ctypes.byref(usage))
    if status != 0:
        raise RetinaAcceptanceError(f"proc_pid_rusage could not sample process {pid} ({status})")
    system = ctypes.CDLL("/usr/lib/libSystem.B.dylib")
    system.mach_timebase_info.argtypes = [ctypes.POINTER(MachTimebaseInfo)]
    system.mach_timebase_info.restype = ctypes.c_int
    timebase = MachTimebaseInfo()
    status = system.mach_timebase_info(ctypes.byref(timebase))
    if status != 0 or timebase.denom == 0:
        raise RetinaAcceptanceError(f"mach_timebase_info failed ({status})")
    absolute = int(usage.ri_user_time + usage.ri_system_time)
    return absolute * int(timebase.numer) // int(timebase.denom)


def _post_key_code(backend: MacOsQuartzBackend, key_code: int, pressed: bool) -> None:
    event = backend._api.CGEventCreateKeyboardEvent(None, key_code, pressed)
    backend._post(event, application_targeted=True)


def _macos_application_is_frontmost(backend: MacOsQuartzBackend, pid: int) -> bool:
    name = backend._cf.CFStringCreateWithCString(None, b"AXFrontmost", 0x08000100)
    application = backend._api.AXUIElementCreateApplication(pid)
    value = ctypes.c_void_p()
    try:
        if not name or not application:
            return False
        status = backend._api.AXUIElementCopyAttributeValue(
            application, name, ctypes.byref(value)
        )
        return bool(
            status == 0
            and value.value
            and backend._cf.CFBooleanGetValue(value)
        )
    finally:
        if value.value:
            backend._cf.CFRelease(value)
        if application:
            backend._cf.CFRelease(application)
        if name:
            backend._cf.CFRelease(name)


def focus_with_retry(
    backend: MacOsQuartzBackend,
    pid: int,
    *,
    process: subprocess.Popen[object] | None = None,
    timeout: float = 10.0,
) -> None:
    """Focus a fullscreen Cocoa window after its Space transition settles."""

    deadline = time.monotonic() + timeout
    last_error: InputError | None = None
    while time.monotonic() < deadline:
        if process is not None and process.poll() is not None:
            raise RetinaAcceptanceError(
                f"client exited with status {process.returncode} while waiting for focus"
            )
        try:
            backend.focus_pid(pid)
            return
        except InputError as error:
            # A GLFW fullscreen window may expose AXMain/AXFocused and become
            # frontmost while declining the redundant AXRaise action. Accept
            # only that exact action-unsupported result, with a fresh native
            # AXFrontmost read proving activation actually succeeded.
            if str(error).endswith("window was not raised (-25206)") and (
                _macos_application_is_frontmost(backend, pid)
            ):
                return
            last_error = error
            time.sleep(0.25)
    raise RetinaAcceptanceError(
        f"could not focus macOS client {pid} within {timeout:.1f}s: {last_error}"
    ) from last_error


def send_screenshot_input(
    backend: MacOsQuartzBackend, pid: int, *, process: subprocess.Popen[object] | None = None
) -> None:
    focus_with_retry(backend, pid, process=process)
    _post_key_code(backend, F2_KEY_CODE, True)
    _post_key_code(backend, F2_KEY_CODE, False)


def send_profiler_input(
    backend: MacOsQuartzBackend, pid: int, *, process: subprocess.Popen[object] | None = None
) -> None:
    focus_with_retry(backend, pid, process=process)
    _post_key_code(backend, F3_KEY_CODE, True)
    _post_key_code(backend, L_KEY_CODE, True)
    time.sleep(0.05)
    _post_key_code(backend, L_KEY_CODE, False)
    _post_key_code(backend, F3_KEY_CODE, False)


def _wait_for_new_file(
    root: Path,
    pattern: str,
    before: set[Path],
    *,
    process: subprocess.Popen[object],
    timeout: float,
) -> Path:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        candidates = set(root.glob(pattern)) - before
        if candidates:
            return max(candidates, key=lambda path: path.stat().st_mtime_ns)
        status = process.poll()
        if status is not None:
            raise RetinaAcceptanceError(
                f"client exited with status {status} before producing {pattern}"
            )
        time.sleep(0.1)
    raise RetinaAcceptanceError(f"timed out waiting for {pattern} below {root}")


def _replace_option(argv: Sequence[str], option: str, value: str) -> tuple[str, ...]:
    result = list(argv)
    try:
        index = result.index(option)
    except ValueError:
        result.extend((option, value))
    else:
        if index + 1 >= len(result):
            raise RetinaAcceptanceError(f"launch option {option} lacks a value")
        result[index + 1] = value
    return tuple(result)


def build_macos_vanilla_argv(
    audit: object,
    *,
    java_path: Path,
    game_dir: Path,
    assets_dir: Path,
    natives_dir: Path,
    server_port: int,
    display: DisplayProbe,
) -> tuple[str, ...]:
    libraries = getattr(audit, "libraries")
    client = getattr(audit, "client")
    main_class = getattr(audit, "main_class")
    asset_index_id = getattr(audit, "asset_index_id")
    classpath = os.pathsep.join([entry.path for entry in libraries] + [client.path])
    return (
        str(java_path),
        "-Xms2G",
        "-Xmx4G",
        "-XX:+UseCompactObjectHeaders",
        "-XX:+AlwaysPreTouch",
        "-XX:+UseStringDeduplication",
        "-XX:+UseZGC",
        "-XstartOnFirstThread",
        "-DMC_DEBUG_ENABLED",
        f"-Djava.library.path={natives_dir}",
        f"-Dorg.lwjgl.librarypath={natives_dir}",
        "-cp",
        classpath,
        str(main_class),
        "--offlineDeveloperMode",
        "--username",
        BENCHMARK_USERNAME,
        "--accessToken",
        "offline",
        "--version",
        MINECRAFT_VERSION,
        "--gameDir",
        str(game_dir),
        "--assetsDir",
        str(assets_dir),
        "--assetIndex",
        str(asset_index_id),
        "--width",
        str(display.logical_width),
        "--height",
        str(display.logical_height),
        "--fullscreen",
        "--quickPlayMultiplayer",
        f"127.0.0.1:{server_port}",
    )


def build_macos_bbb_argv(
    *,
    binary: Path,
    vanilla_root: Path,
    game_dir: Path,
    server_port: int,
    control_port: int,
    display: DisplayProbe,
) -> tuple[str, ...]:
    argv = build_bbb_launch_argv(
        binary=binary,
        vanilla_root=vanilla_root,
        server_port=server_port,
        control_port=control_port,
    )
    for option, value in (
        ("--width", str(display.backing_pixel_width)),
        ("--height", str(display.backing_pixel_height)),
        ("--client-vsync", "false"),
        ("--client-framerate-limit", str(UNLIMITED_FPS_SENTINEL)),
    ):
        argv = _replace_option(argv, option, value)
    return (
        *argv,
        "--game-dir",
        str(game_dir),
        "--client-fullscreen",
        "true",
    )


def _adapter_from_log(client: str, text: str, fallback: str) -> dict[str, str]:
    if client == "bbb":
        name = re.search(r'adapter_name=(?:"([^"]+)"|([^ ]+))', text)
        backend = re.search(r'adapter_backend=(?:"([^"]+)"|([^ ]+))', text)
        return {
            "device": (name.group(1) or name.group(2)) if name else fallback,
            "api": (backend.group(1) or backend.group(2)) if backend else "metal",
        }
    renderer = re.search(r"(?im)^\s*(?:OpenGL renderer|Renderer):\s*(.+)$", text)
    return {
        "device": renderer.group(1).strip() if renderer else fallback,
        "api": "opengl",
    }


def _write_profile_samples(
    path: Path, profile_path: Path, segment: str, cpu_nanos: int
) -> dict[str, object]:
    samples = profiler_samples(profile_path)
    entry = parse_profiler_archive(profile_path)
    entry["segment"] = segment
    for metric, milliseconds in samples.items():
        output = path / f"{segment}-{metric}-samples.json"
        output.write_bytes(canonical_json_bytes({"milliseconds": milliseconds}))
        entry[f"{metric}_samples"] = str(output)
    frame_count = len(samples["application_frame_time"])
    entry["process_cpu_time_ms"] = cpu_nanos / 1_000_000.0
    entry["process_cpu_frame_time"] = {
        "frames": frame_count,
        "mean_ms": cpu_nanos / 1_000_000.0 / frame_count,
        "scope": (
            "proc_pid_rusage user+system CPU delta across the F3+L window, "
            "normalized by application-frame samples"
        ),
    }
    return entry


def _run_client(
    *,
    client: str,
    display: DisplayProbe,
    replay: Mapping[str, object],
    vanilla_root: Path,
    java_path: Path,
    bbb_binary: Path,
    world_manifest_path: Path,
    artifact_dir: Path,
    audit: object,
    natives_dir: Path,
    timeout: float,
) -> dict[str, object]:
    if client not in {"vanilla", "bbb"}:
        raise RetinaAcceptanceError(f"unsupported formal client {client!r}")
    if artifact_dir.exists() and any(artifact_dir.iterdir()):
        raise RetinaAcceptanceError(f"run directory is not empty: {artifact_dir}")
    artifact_dir.mkdir(parents=True, exist_ok=True)
    source_manifest = load_json_object(world_manifest_path, "world snapshot manifest")
    source_world = Path(str(source_manifest.get("world", "")))
    _verify_world_snapshot(world_manifest_path, source_world)
    server_port = reserve_loopback_port()
    control_port = reserve_loopback_port()
    server_dir = artifact_dir / "server"
    shutil.copytree(source_world, server_dir / "world")
    digest = write_server_layout(
        server_dir,
        replay=replay,
        server_port=server_port,
        accept_eula=True,
    )
    if digest != source_manifest.get("replay_hash"):
        raise RetinaAcceptanceError("world snapshot and replay identities differ")
    game_dir = artifact_dir / "game"
    options_path = write_vanilla_options(game_dir)
    server = validate_server_jar(vanilla_root)
    if client == "vanilla":
        client_argv = build_macos_vanilla_argv(
            audit,
            java_path=java_path,
            game_dir=game_dir,
            assets_dir=vanilla_root / "downloads" / MINECRAFT_VERSION / "assets",
            natives_dir=natives_dir,
            server_port=server_port,
            display=display,
        )
        profile_root = game_dir / "debug" / "profiling"
    else:
        client_argv = build_macos_bbb_argv(
            binary=bbb_binary,
            vanilla_root=vanilla_root,
            game_dir=game_dir,
            server_port=server_port,
            control_port=control_port,
            display=display,
        )
        profile_root = artifact_dir / "debug" / "profiling"
    screenshot_root = game_dir / "screenshots"
    settings = settings_identity()
    input_script = {
        "events": ["F2", "F3-down", "L-down", "L-up", "F3-up"],
        "at_markers": [f"{segment}:capture_start" for segment in CAPTURE_SEGMENTS],
        "targeting": "CoreGraphics keyboard events posted to exact client pid",
    }
    result: dict[str, object] = {
        "schema": SCHEMA,
        "profile": PROFILE_ID,
        "client": client,
        "status": "started",
        "window_mode": "fullscreen",
        "logical_size": [display.logical_width, display.logical_height],
        "target_drawable_pixels": list(display.target_drawable),
        "drawable_pixels": None,
        "display_id": display.display_id,
        "scale_factor": display.scale_factor,
        "vsync": False,
        "framerate_limit": "unlimited",
        "framerate_limit_sentinel": UNLIMITED_FPS_SENTINEL,
        "warmup_seconds": WARMUP_SECONDS,
        "replay_sha256": canonical_sha256(replay),
        "world_sha256": str(source_manifest["world_hash"]),
        "settings_sha256": str(settings["sha256"]),
        "resource_pack_sha256": str(getattr(audit, "resource_hash")),
        "input_script_sha256": canonical_sha256(input_script),
        "input_script": input_script,
        "screenshots": [],
        "profiles": [],
        "server_port": server_port,
        "control_port": control_port if client == "bbb" else None,
        "launch_argv": sanitized_argv(client_argv),
        "client_log": str(artifact_dir / f"{client}-client.log"),
        "server_log": str(artifact_dir / "server.log"),
    }
    evidence_path = artifact_dir / "run.json"
    server_group: ProcessGroup | None = None
    client_group: ProcessGroup | None = None
    server_log_path = Path(str(result["server_log"]))
    client_log_path = Path(str(result["client_log"]))
    try:
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
            environment.update(
                {
                    "BBB_VANILLA_ROOT": str(vanilla_root),
                    "RUST_LOG": "bbb_native=info,bbb_renderer=info,bbb_net=info",
                }
            )
            client_group = ProcessGroup(
                client_argv,
                cwd=artifact_dir if client == "bbb" else game_dir,
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
            input_backend = MacOsQuartzBackend(client_process.pid)
            focus_with_retry(input_backend, client_process.pid, process=client_process)
            _warmup_and_start_replay(server_process, replay)
            starts = [
                marker
                for marker in replay_capture_markers(replay)
                if marker["marker"] == "capture_start"
            ]
            for marker in starts:
                segment = str(marker["capture_segment"])
                _wait_for_log(
                    server_log_path,
                    benchmark_marker_line(digest, marker),
                    process=client_process,
                    timeout=timeout,
                )
                before_screenshots = set(screenshot_root.glob("*.png"))
                before_profiles = set(profile_root.glob("*.zip"))
                send_screenshot_input(
                    input_backend, client_process.pid, process=client_process
                )
                screenshot = _wait_for_new_file(
                    screenshot_root,
                    "*.png",
                    before_screenshots,
                    process=client_process,
                    timeout=10.0,
                )
                cpu_before = process_cpu_time_nanos(client_process.pid)
                send_profiler_input(input_backend, client_process.pid, process=client_process)
                profile = _wait_for_new_file(
                    profile_root,
                    "*.zip",
                    before_profiles,
                    process=client_process,
                    timeout=PROFILE_SECONDS + 10.0,
                )
                cpu_after = process_cpu_time_nanos(client_process.pid)
                dimensions = png_dimensions(screenshot)
                result["screenshots"].append(
                    {
                        "segment": segment,
                        "path": str(screenshot),
                        "sha256": file_digest(screenshot, "sha256"),
                        "pixels": list(dimensions),
                    }
                )
                result["profiles"].append(
                    _write_profile_samples(
                        artifact_dir,
                        profile,
                        segment,
                        cpu_after - cpu_before,
                    )
                )
            final_marker = replay_capture_markers(replay)[-1]
            _wait_for_log(
                server_log_path,
                benchmark_marker_line(digest, final_marker),
                process=client_process,
                timeout=timeout,
            )
            observed = {
                tuple(item["pixels"]) for item in result["screenshots"] if isinstance(item, Mapping)
            }
            result["drawable_pixels"] = list(next(iter(observed))) if len(observed) == 1 else None
            result["status"] = "passed"
    except (BenchmarkFailure, InputError, RetinaAcceptanceError, OSError) as error:
        result["status"] = "failed"
        result["failure"] = str(error)
    finally:
        if client_group is not None:
            try:
                client_group.cleanup(timeout=10.0)
            except BenchmarkFailure as error:
                result.setdefault("cleanup_failures", []).append(str(error))
        if server_group is not None and server_group.process is not None:
            process = server_group.process
            if process.poll() is None:
                try:
                    _write_server_commands(process, ("stop",))
                    process.wait(timeout=20.0)
                except (BenchmarkFailure, subprocess.TimeoutExpired):
                    pass
            try:
                server_group.cleanup(timeout=5.0)
            except BenchmarkFailure as error:
                result.setdefault("cleanup_failures", []).append(str(error))
        client_text = (
            client_log_path.read_text(encoding="utf-8", errors="replace")
            if client_log_path.is_file()
            else ""
        )
        result["adapter"] = _adapter_from_log(client, client_text, display.metal_device)
        if options_path.is_file():
            try:
                result["loaded_options"] = audit_vanilla_options(options_path)
            except BenchmarkFailure as error:
                result.setdefault("validation_failures", []).append(str(error))
                result["status"] = "failed"
        evidence_path.write_bytes(canonical_json_bytes(result))
    return result


def run_formal(
    *,
    artifact_root: Path,
    vanilla_root: Path,
    java_path: Path,
    bbb_binary: Path,
    replay_path: Path,
    timeout: float,
) -> dict[str, object]:
    if artifact_root.exists() and any(artifact_root.iterdir()):
        raise RetinaAcceptanceError(f"formal artifact root is not empty: {artifact_root}")
    capture_commit = require_clean_full_gated_head()
    artifact_root.mkdir(parents=True, exist_ok=True)
    profile_contract = load_profile_contract()
    display = probe_builtin_display()
    replay = load_json_object(replay_path, "formal replay")
    audit = audit_macos_distribution(vanilla_root, java_path=java_path)
    natives_dir = artifact_root / "natives"
    natives = safe_extract_macos_natives(
        tuple(Path(entry.path) for entry in audit.native_classifiers), natives_dir
    )
    preparation_dir = artifact_root / "world-preparation"
    snapshot = prepare_world_snapshot(
        replay=replay,
        vanilla_root=vanilla_root,
        java_path=java_path,
        artifact_dir=preparation_dir,
        server_port=reserve_loopback_port(),
        accept_eula=True,
        timeout=timeout,
    )
    world_manifest = preparation_dir / "world-snapshot.json"
    run_paths: list[str] = []
    runs: list[dict[str, object]] = []
    manifest_path = artifact_root / "formal-manifest.json"
    for sequence, client in enumerate(FORMAL_ORDER, start=1):
        run_dir = artifact_root / f"run-{sequence:02d}-{client}"
        result = _run_client(
            client=client,
            display=display,
            replay=replay,
            vanilla_root=vanilla_root,
            java_path=java_path,
            bbb_binary=bbb_binary,
            world_manifest_path=world_manifest,
            artifact_dir=run_dir,
            audit=audit,
            natives_dir=natives_dir,
            timeout=timeout,
        )
        run_path = run_dir / "run.json"
        run_paths.append(str(run_path))
        runs.append(result)
        manifest_path.write_bytes(
            canonical_json_bytes(
                {
                    "schema": SCHEMA,
                    "profile": PROFILE_ID,
                    "bbb_commit": capture_commit,
                    "profile_contract": str(PROFILE_CONTRACT_PATH),
                    "profile_contract_sha256": file_digest(PROFILE_CONTRACT_PATH, "sha256"),
                    "display": asdict(display),
                    "replay": str(replay_path),
                    "replay_sha256": file_digest(replay_path, "sha256"),
                    "world_snapshot": asdict(snapshot),
                    "natives": [asdict(entry) for entry in natives],
                    "runs": run_paths,
                }
            )
        )
        if result.get("status") != "passed" or result.get("drawable_pixels") != list(
            display.target_drawable
        ):
            break
    report = validate_and_summarize(display, runs)
    report.update(
        {
            "artifact_root": str(artifact_root),
            "bbb_commit": capture_commit,
            "profile_contract": profile_contract,
            "profile_contract_path": str(PROFILE_CONTRACT_PATH),
            "profile_contract_sha256": file_digest(PROFILE_CONTRACT_PATH, "sha256"),
            "formal_manifest": str(manifest_path),
            "formal_manifest_sha256": file_digest(manifest_path, "sha256"),
            "replay": str(replay_path),
            "replay_sha256": file_digest(replay_path, "sha256"),
            "world_snapshot": asdict(snapshot),
            "bbb_binary": str(bbb_binary),
            "bbb_binary_sha256": file_digest(bbb_binary, "sha256"),
        }
    )
    report_path = artifact_root / "report.json"
    report_path.write_bytes(canonical_json_bytes(report))
    return report


def _detect_command(args: argparse.Namespace) -> int:
    display = probe_builtin_display()
    profile_contract = load_profile_contract()
    report = {
        "schema": SCHEMA,
        "profile": PROFILE_ID,
        "profile_contract": profile_contract,
        "profile_contract_path": str(PROFILE_CONTRACT_PATH),
        "profile_contract_sha256": file_digest(PROFILE_CONTRACT_PATH, "sha256"),
        "host": platform.uname()._asdict(),
        "display": asdict(display),
        "launch_contract": launch_contract(
            display, server_port=args.server_port, control_port=args.control_port
        ),
        "settings": settings_identity(),
    }
    payload = canonical_json_bytes(report)
    if args.output is None:
        sys.stdout.buffer.write(payload)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(payload)
    return 0


def _summarize_command(args: argparse.Namespace) -> int:
    manifest = _load_object(args.manifest, "formal manifest")
    display_value = manifest.get("display")
    run_paths = manifest.get("runs")
    if not isinstance(display_value, dict) or not isinstance(run_paths, list):
        raise RetinaAcceptanceError("formal manifest lacks display or runs")
    display = DisplayProbe(**display_value)
    runs = [_load_object(Path(str(path)), "formal run") for path in run_paths]
    report = validate_and_summarize(display, runs)
    report["manifest"] = str(args.manifest)
    report["manifest_sha256"] = hashlib.sha256(args.manifest.read_bytes()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_json_bytes(report))
    return 0 if report["status"] == "passed" else 2


def _run_command(args: argparse.Namespace) -> int:
    report = run_formal(
        artifact_root=args.artifact_root,
        vanilla_root=args.vanilla_root,
        java_path=args.java,
        bbb_binary=args.bbb_binary,
        replay_path=args.replay,
        timeout=args.timeout,
    )
    return 0 if report["status"] == "passed" else 2


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    detect = subparsers.add_parser("detect")
    detect.add_argument("--output", type=Path)
    detect.add_argument("--server-port", type=int, default=25565)
    detect.add_argument("--control-port", type=int, default=24454)
    summarize = subparsers.add_parser("summarize")
    summarize.add_argument("--manifest", type=Path, required=True)
    summarize.add_argument("--output", type=Path, required=True)
    run = subparsers.add_parser("run")
    run.add_argument("--artifact-root", type=Path, required=True)
    run.add_argument("--vanilla-root", type=Path, required=True)
    run.add_argument("--java", type=Path, default=Path("/usr/bin/java"))
    run.add_argument("--bbb-binary", type=Path, required=True)
    run.add_argument(
        "--replay",
        type=Path,
        default=(
            Path(__file__).resolve().parent
            / "fixtures"
            / "official-client-benchmark"
            / "replay-26.1.json"
        ),
    )
    run.add_argument("--timeout", type=float, default=180.0)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        if args.command == "detect":
            return _detect_command(args)
        if args.command == "summarize":
            return _summarize_command(args)
        if args.command == "run":
            return _run_command(args)
        raise AssertionError(args.command)
    except RetinaAcceptanceError as error:
        print(f"macos-native-retina: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
