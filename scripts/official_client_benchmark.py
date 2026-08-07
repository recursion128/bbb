#!/usr/bin/env python3
"""Official Minecraft 26.1 equal-load benchmark support.

This module intentionally keeps network downloads outside the benchmark path.
It validates a provisioned Mojang distribution, prepares an unmodified offline
client launch, generates the server-authoritative camera replay datapack, and
parses common external capture artifacts.
"""

from __future__ import annotations

import argparse
import binascii
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
import struct
import subprocess
import sys
import time
import xml.etree.ElementTree as ElementTree
import zipfile
import zlib
from dataclasses import asdict, dataclass
from pathlib import Path, PurePosixPath
from typing import Any, Iterable, Mapping, Sequence

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_workspace
from bbb_tools.vanilla.lockfile import load_lock
from bbb_tools.execution.process import ManagedProcess, ProcessFailure


BENCHMARK_SCHEMA = 1
REPLAY_SCHEMA = 1
MINECRAFT_VERSION = "26.1"
BENCHMARK_USERNAME = "bbb-benchmark"
VANILLA_WINDOW_TITLE = f"Minecraft {MINECRAFT_VERSION}"
BBB_WINDOW_TITLE = f"Minecraft* {MINECRAFT_VERSION} - Multiplayer"
OFFLINE_ACCESS_TOKEN = "offline"
WINDOW_WIDTH = 1280
WINDOW_HEIGHT = 720
COMPOSITOR_WIDTH = 1600
COMPOSITOR_HEIGHT = 900
MAX_VISUAL_PNG_BYTES = 16 * 1024 * 1024
MAX_VISUAL_PNG_PIXELS = COMPOSITOR_WIDTH * COMPOSITOR_HEIGHT
VIEW_DISTANCE = 12
SIMULATION_DISTANCE = 12
SERVER_SEED = "bbb-official-benchmark-26.1"
VOID_FLAT_GENERATOR_SETTINGS = json.dumps(
    {
        "biome": "minecraft:plains",
        "features": False,
        "lakes": False,
        "layers": [{"block": "minecraft:air", "height": 1}],
        "structure_overrides": [],
    },
    sort_keys=True,
    separators=(",", ":"),
)
SERVER_READY_MARKER = "Done ("
WORLD_PREPARED_MARKER = "BBB_BENCHMARK_WORLD_PREPARED"
DEFAULT_HITCH_THRESHOLDS_MS = (33.333, 50.0, 100.0)
THERMAL_REJECT_C = 95.0
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
    # Upstream renamed the `fence:` tracepoint subsystem to `dma_fence:` when the
    # fence API became dma_fence; this is the same completion event, not a
    # substitute for a missing one.
    "dma_fence:dma_fence_signaled",
    "drm:drm_vblank_event",
    "sched:sched_switch",
)
# The submission -> dispatch -> completion chain that yields per-client GPU
# execution duration. It sits in the amdgpu kernel driver, below both radeonsi
# (the official client's OpenGL) and RADV (bbb's Vulkan), so one capture covers
# both APIs without instrumenting either client. `scripts/gpu_duration_capture.py`
# owns the join and the reporting.
GPU_DURATION_CHAIN_EVENTS = (
    "amdgpu:amdgpu_cs_ioctl",
    "amdgpu:amdgpu_sched_run_job",
    "dma_fence:dma_fence_signaled",
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

# The managed Weston fixture deterministically places the 1280x720 client
# content at this compositor coordinate. Pixel evidence rejects any capture
# that no longer satisfies the frozen 1600x900/1280x720 contract.
WESTON_CLIENT_CONTENT_ORIGIN = (102, 178)
VISUAL_PIXEL_THRESHOLDS = {
    "max_mean_channel_delta": 8.0,
    "max_neighborhood_mae": 12.0,
    "max_severe_neighborhood_fraction": 0.12,
    "max_edge_density_delta": 0.03,
}
# Coordinates are relative to the client content, not the compositor. Regions
# intentionally avoid the Vanilla debug text while covering every material in
# the P0 acceptance matrix. They are frozen before formal cross-client runs.
VISUAL_ROIS = {
    "static_orbit": (
        ("water", (400, 500, 850, 700)),
        ("ordinary_glass", (800, 330, 1220, 520)),
        ("glass_water_boundary", (20, 430, 360, 650)),
        ("armor_stands", (450, 430, 850, 650)),
    ),
    "preloaded_flight": (
        ("leaves", (500, 470, 790, 710)),
        ("stained_glass", (850, 390, 1050, 690)),
        ("stone", (550, 120, 760, 390)),
        ("glowstone", (20, 310, 500, 560)),
    ),
    "chunk_boundary_crossing": (
        ("glass_chunk_boundary", (1030, 40, 1270, 700)),
        ("stone_chunk_boundary", (420, 80, 980, 600)),
        ("glowstone", (20, 330, 370, 580)),
    ),
}
FORMAL_CAPTURE_SEGMENTS = tuple(VISUAL_ROIS)


class BenchmarkFailure(RuntimeError):
    """A benchmark contract failure with actionable text."""


@dataclass(frozen=True)
class RgbImage:
    width: int
    height: int
    rgb: bytes


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
    gpu_temp_peak_c: float | None
    cpu_temp_mean_c: float | None
    cpu_temp_peak_c: float | None


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
        "particles": "minimal",
        "mipmap_levels": 4,
        "cutout_leaves": True,
        "entity_shadows": True,
        "improved_transparency": True,
        "weather_radius": 10,
        "max_anisotropy_bit": 1,
        "texture_filtering": "rgss",
        "hud_hidden": False,
        "gui_scale": 1,
        "force_unicode_font": False,
        "fov_degrees": 70,
        "gamma": 0.5,
        "view_bobbing": True,
        "pause_on_lost_focus": False,
        "audio": "disabled",
        "vsync": False,
        "framerate_limit": "unlimited",
        "inactivity_fps_limit": "minimized",
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


def _png_chunks(payload: bytes) -> Iterable[tuple[bytes, bytes]]:
    if not payload.startswith(b"\x89PNG\r\n\x1a\n"):
        raise BenchmarkFailure("visual evidence is not a PNG file")
    offset = 8
    while offset < len(payload):
        if offset + 12 > len(payload):
            raise BenchmarkFailure("PNG chunk header is truncated")
        size = struct.unpack(">I", payload[offset : offset + 4])[0]
        kind = payload[offset + 4 : offset + 8]
        end = offset + 12 + size
        if end > len(payload):
            raise BenchmarkFailure("PNG chunk payload is truncated")
        data = payload[offset + 8 : offset + 8 + size]
        expected_crc = struct.unpack(">I", payload[offset + 8 + size : end])[0]
        actual_crc = binascii.crc32(kind + data) & 0xFFFFFFFF
        if actual_crc != expected_crc:
            raise BenchmarkFailure(f"PNG {kind!r} CRC mismatch")
        yield kind, data
        offset = end
        if kind == b"IEND":
            if offset != len(payload):
                raise BenchmarkFailure("PNG has trailing data after IEND")
            return
    raise BenchmarkFailure("PNG lacks IEND")


def read_png_rgb(path: Path) -> RgbImage:
    try:
        with path.open("rb") as source:
            payload = source.read(MAX_VISUAL_PNG_BYTES + 1)
    except OSError as error:
        raise BenchmarkFailure(f"cannot read visual evidence PNG {path}: {error}") from error
    if len(payload) > MAX_VISUAL_PNG_BYTES:
        raise BenchmarkFailure(
            f"visual evidence PNG exceeds {MAX_VISUAL_PNG_BYTES} bytes"
        )
    width = height = None
    idat: list[bytes] = []
    for kind, data in _png_chunks(payload):
        if kind == b"IHDR":
            if width is not None or len(data) != 13:
                raise BenchmarkFailure("PNG has an invalid IHDR")
            width, height, depth, color, compression, filtering, interlace = struct.unpack(
                ">IIBBBBB", data
            )
            if not width or not height:
                raise BenchmarkFailure("PNG dimensions must be non-zero")
            if width * height > MAX_VISUAL_PNG_PIXELS:
                raise BenchmarkFailure(
                    "visual evidence PNG dimensions exceed the compositor pixel limit"
                )
            if (depth, color, compression, filtering, interlace) != (8, 2, 0, 0, 0):
                raise BenchmarkFailure(
                    "visual evidence PNG must be non-interlaced 8-bit RGB"
                )
        elif kind == b"IDAT":
            idat.append(data)
    if width is None or height is None or not idat:
        raise BenchmarkFailure("PNG lacks IHDR or IDAT")
    stride = width * 3
    expected = height * (stride + 1)
    try:
        decompressor = zlib.decompressobj()
        filtered = decompressor.decompress(b"".join(idat), expected + 1)
    except zlib.error as error:
        raise BenchmarkFailure(f"PNG IDAT decompression failed: {error}") from error
    if (
        len(filtered) != expected
        or not decompressor.eof
        or decompressor.unconsumed_tail
        or decompressor.unused_data
    ):
        raise BenchmarkFailure(
            f"PNG scanline size mismatch: expected {expected}, got {len(filtered)}"
        )
    output = bytearray(width * height * 3)
    previous = bytearray(stride)
    source_offset = 0
    for y in range(height):
        filter_type = filtered[source_offset]
        source_offset += 1
        current = bytearray(filtered[source_offset : source_offset + stride])
        source_offset += stride
        if filter_type > 4:
            raise BenchmarkFailure(f"PNG uses invalid filter {filter_type}")
        for index in range(stride):
            left = current[index - 3] if index >= 3 else 0
            above = previous[index]
            upper_left = previous[index - 3] if index >= 3 else 0
            if filter_type == 0:
                predictor = 0
            elif filter_type == 1:
                predictor = left
            elif filter_type == 2:
                predictor = above
            elif filter_type == 3:
                predictor = (left + above) // 2
            else:
                estimate = left + above - upper_left
                left_error = abs(estimate - left)
                above_error = abs(estimate - above)
                upper_left_error = abs(estimate - upper_left)
                predictor = (
                    left
                    if left_error <= above_error and left_error <= upper_left_error
                    else above
                    if above_error <= upper_left_error
                    else upper_left
                )
            current[index] = (current[index] + predictor) & 0xFF
        start = y * stride
        output[start : start + stride] = current
        previous = current
    return RgbImage(width=width, height=height, rgb=bytes(output))


def _png_chunk(kind: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + kind
        + payload
        + struct.pack(">I", binascii.crc32(kind + payload) & 0xFFFFFFFF)
    )


def write_png_rgb(path: Path, image: RgbImage) -> None:
    expected = image.width * image.height * 3
    if image.width <= 0 or image.height <= 0 or len(image.rgb) != expected:
        raise BenchmarkFailure("RGB image dimensions do not match its payload")
    stride = image.width * 3
    scanlines = bytearray()
    for y in range(image.height):
        scanlines.append(0)
        start = y * stride
        scanlines.extend(image.rgb[start : start + stride])
    payload = b"".join(
        (
            b"\x89PNG\r\n\x1a\n",
            _png_chunk(
                b"IHDR",
                struct.pack(">IIBBBBB", image.width, image.height, 8, 2, 0, 0, 0),
            ),
            _png_chunk(b"IDAT", zlib.compress(bytes(scanlines), level=9)),
            _png_chunk(b"IEND", b""),
        )
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload)


def _pixel(image: RgbImage, x: int, y: int) -> tuple[int, int, int]:
    offset = (y * image.width + x) * 3
    return (image.rgb[offset], image.rgb[offset + 1], image.rgb[offset + 2])


def _roi_directional_neighborhood_metrics(
    source: RgbImage,
    target: RgbImage,
    bounds: tuple[int, int, int, int],
) -> tuple[float, float]:
    x0, y0, x1, y1 = bounds
    distance_sum = 0.0
    severe = 0
    samples = (x1 - x0) * (y1 - y0)
    for y in range(y0, y1):
        for x in range(x0, x1):
            source_pixel = _pixel(source, x, y)
            best = float("inf")
            for target_y in range(max(y0, y - 1), min(y1, y + 2)):
                for target_x in range(max(x0, x - 1), min(x1, x + 2)):
                    target_pixel = _pixel(target, target_x, target_y)
                    distance = sum(
                        abs(source_pixel[channel] - target_pixel[channel])
                        for channel in range(3)
                    ) / 3.0
                    best = min(best, distance)
            distance_sum += best
            severe += best > 32.0
    return distance_sum / samples, severe / samples


def _roi_edge_density(image: RgbImage, bounds: tuple[int, int, int, int]) -> float:
    x0, y0, x1, y1 = bounds
    edges = 0
    samples = max(1, (x1 - x0 - 1) * (y1 - y0 - 1))

    def luminance(x: int, y: int) -> int:
        red, green, blue = _pixel(image, x, y)
        return (54 * red + 183 * green + 19 * blue) // 256

    for y in range(y0, y1 - 1):
        for x in range(x0, x1 - 1):
            value = luminance(x, y)
            if max(
                abs(value - luminance(x + 1, y)),
                abs(value - luminance(x, y + 1)),
            ) > 20:
                edges += 1
    return edges / samples


def compare_visual_roi(
    vanilla: RgbImage,
    bbb: RgbImage,
    bounds: tuple[int, int, int, int],
) -> dict[str, Any]:
    x0, y0, x1, y1 = bounds
    if not (0 <= x0 < x1 <= vanilla.width and 0 <= y0 < y1 <= vanilla.height):
        raise BenchmarkFailure(f"visual ROI is outside the capture: {bounds}")
    if (vanilla.width, vanilla.height) != (bbb.width, bbb.height):
        raise BenchmarkFailure("visual evidence image dimensions differ")
    samples = (x1 - x0) * (y1 - y0)
    vanilla_sum = [0, 0, 0]
    bbb_sum = [0, 0, 0]
    raw_difference_sum = 0
    for y in range(y0, y1):
        for x in range(x0, x1):
            left = _pixel(vanilla, x, y)
            right = _pixel(bbb, x, y)
            for channel in range(3):
                vanilla_sum[channel] += left[channel]
                bbb_sum[channel] += right[channel]
                raw_difference_sum += abs(left[channel] - right[channel])
    mean_channel_delta = max(
        abs(vanilla_sum[channel] - bbb_sum[channel]) / samples
        for channel in range(3)
    )
    forward_mae, forward_severe = _roi_directional_neighborhood_metrics(
        vanilla, bbb, bounds
    )
    reverse_mae, reverse_severe = _roi_directional_neighborhood_metrics(
        bbb, vanilla, bounds
    )
    neighborhood_mae = max(forward_mae, reverse_mae)
    severe_fraction = max(forward_severe, reverse_severe)
    edge_density_delta = abs(
        _roi_edge_density(vanilla, bounds) - _roi_edge_density(bbb, bounds)
    )
    metrics = {
        "samples": samples,
        "vanilla_mean_rgb": [round(value / samples, 6) for value in vanilla_sum],
        "bbb_mean_rgb": [round(value / samples, 6) for value in bbb_sum],
        "mean_channel_delta": round(mean_channel_delta, 6),
        "raw_mae": round(raw_difference_sum / (samples * 3), 6),
        "neighborhood_mae": round(neighborhood_mae, 6),
        "severe_neighborhood_fraction": round(severe_fraction, 6),
        "edge_density_delta": round(edge_density_delta, 6),
    }
    failures = []
    for metric, threshold in VISUAL_PIXEL_THRESHOLDS.items():
        field = metric.removeprefix("max_")
        if metrics[field] > threshold:
            failures.append(
                f"{field}={metrics[field]} exceeds frozen threshold {threshold}"
            )
    metrics["status"] = "passed" if not failures else "failed"
    metrics["failures"] = failures
    return metrics


def analyze_visual_anchor_pairs(
    screenshots: Mapping[str, Mapping[str, Mapping[str, Any]]],
    artifact_dir: Path,
) -> dict[str, Any]:
    artifact_dir.mkdir(parents=True, exist_ok=True)
    origin_x, origin_y = WESTON_CLIENT_CONTENT_ORIGIN
    region_results = []
    diff_maps = []
    failures = []
    for segment, rois in VISUAL_ROIS.items():
        pair = screenshots.get(segment)
        if not isinstance(pair, Mapping):
            raise BenchmarkFailure(f"visual evidence lacks {segment} screenshot pair")
        images = {}
        for client in ("vanilla", "bbb"):
            screenshot = pair.get(client)
            if not isinstance(screenshot, Mapping):
                raise BenchmarkFailure(f"visual evidence lacks {client}/{segment}")
            path = Path(str(screenshot.get("path", "")))
            if file_digest(path, "sha256") != screenshot.get("sha256"):
                raise BenchmarkFailure(f"{client} {segment} screenshot SHA-256 mismatch")
            image = read_png_rgb(path)
            if (image.width, image.height) != (
                COMPOSITOR_WIDTH,
                COMPOSITOR_HEIGHT,
            ):
                raise BenchmarkFailure(
                    f"{client} {segment} compositor capture dimensions mismatch"
                )
            images[client] = image
        content_bounds = (
            origin_x,
            origin_y,
            origin_x + WINDOW_WIDTH,
            origin_y + WINDOW_HEIGHT,
        )
        difference = bytearray(WINDOW_WIDTH * WINDOW_HEIGHT * 3)
        for content_y in range(WINDOW_HEIGHT):
            for content_x in range(WINDOW_WIDTH):
                vanilla_pixel = _pixel(
                    images["vanilla"], origin_x + content_x, origin_y + content_y
                )
                bbb_pixel = _pixel(
                    images["bbb"], origin_x + content_x, origin_y + content_y
                )
                offset = (content_y * WINDOW_WIDTH + content_x) * 3
                for channel in range(3):
                    difference[offset + channel] = min(
                        255,
                        abs(vanilla_pixel[channel] - bbb_pixel[channel]) * 4,
                    )
        diff_path = artifact_dir / f"{segment}-pixel-diff.png"
        write_png_rgb(
            diff_path,
            RgbImage(WINDOW_WIDTH, WINDOW_HEIGHT, bytes(difference)),
        )
        diff_maps.append(
            {
                "capture_segment": segment,
                "path": str(diff_path),
                "sha256": file_digest(diff_path, "sha256"),
            }
        )
        for material, relative in rois:
            relative_x0, relative_y0, relative_x1, relative_y1 = relative
            bounds = (
                origin_x + relative_x0,
                origin_y + relative_y0,
                origin_x + relative_x1,
                origin_y + relative_y1,
            )
            if not (
                content_bounds[0] <= bounds[0] < bounds[2] <= content_bounds[2]
                and content_bounds[1] <= bounds[1] < bounds[3] <= content_bounds[3]
            ):
                raise BenchmarkFailure(f"{segment}/{material} ROI leaves client content")
            result = compare_visual_roi(images["vanilla"], images["bbb"], bounds)
            result.update(
                {
                    "capture_segment": segment,
                    "material": material,
                    "client_relative_bounds": list(relative),
                }
            )
            region_results.append(result)
            failures.extend(
                f"{segment}/{material}: {failure}" for failure in result["failures"]
            )
    return {
        "status": "passed" if not failures else "failed",
        "content_origin": list(WESTON_CLIENT_CONTENT_ORIGIN),
        "content_resolution": [WINDOW_WIDTH, WINDOW_HEIGHT],
        "thresholds": dict(VISUAL_PIXEL_THRESHOLDS),
        "regions": region_results,
        "diff_maps": diff_maps,
        "failures": failures,
    }


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


def validate_bbb_binary_provenance(
    provenance: Mapping[str, Any],
    *,
    expected_capture_commit: str | None,
    expected_capture_sha256: str,
) -> dict[str, Any]:
    """Validate that a later evidence commit rebuilds the captured BBB binary.

    Formal captures remain attributable to the exact commit that produced them.
    Evidence-only tooling or documentation may advance afterwards, but only an
    independently rebuilt byte-identical binary may bind those captures to the
    later evidence commit.  Both identities stay explicit in the receipt.
    """
    if provenance.get("schema") != BENCHMARK_SCHEMA:
        raise BenchmarkFailure("BBB binary provenance schema mismatch")
    capture_commit = provenance.get("capture_commit")
    evidence_commit = provenance.get("evidence_commit")
    for label, commit in (
        ("capture", capture_commit),
        ("evidence", evidence_commit),
    ):
        if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
            raise BenchmarkFailure(f"BBB {label} commit is not an exact Git commit")
    if expected_capture_commit is not None and capture_commit != expected_capture_commit:
        raise BenchmarkFailure("BBB provenance capture commit mismatch")

    binaries: dict[str, dict[str, Any]] = {}
    for label in ("capture", "evidence"):
        entry = provenance.get(f"{label}_binary")
        if not isinstance(entry, Mapping):
            raise BenchmarkFailure(f"BBB provenance lacks the {label} binary")
        path_value = entry.get("path")
        declared_sha256 = entry.get("sha256")
        if not isinstance(path_value, str) or not Path(path_value).is_absolute():
            raise BenchmarkFailure(f"BBB {label} binary path is not absolute")
        if not isinstance(declared_sha256, str) or re.fullmatch(
            r"[0-9a-f]{64}", declared_sha256
        ) is None:
            raise BenchmarkFailure(f"BBB {label} binary SHA-256 is invalid")
        path = Path(path_value)
        if not path.is_file():
            raise BenchmarkFailure(f"BBB {label} binary is unavailable: {path}")
        actual_sha256 = file_digest(path, "sha256")
        if actual_sha256 != declared_sha256:
            raise BenchmarkFailure(
                f"BBB {label} binary SHA-256 mismatch: "
                f"expected {declared_sha256}, got {actual_sha256}"
            )
        binaries[label] = {
            "path": str(path),
            "sha256": actual_sha256,
            "bytes": path.stat().st_size,
        }
    if binaries["capture"]["sha256"] != expected_capture_sha256:
        raise BenchmarkFailure("BBB captured binary does not match the capture receipt")
    if binaries["capture"]["sha256"] != binaries["evidence"]["sha256"]:
        raise BenchmarkFailure("BBB evidence commit binary is not byte-identical to capture")
    return {
        "capture_commit": capture_commit,
        "evidence_commit": evidence_commit,
        "binary_sha256": binaries["capture"]["sha256"],
        "binary_identical": True,
        "capture_binary": binaries["capture"],
        "evidence_binary": binaries["evidence"],
        "scope": (
            "captures belong to capture_commit; evidence_commit is accepted only "
            "because its independently rebuilt bbb-native binary is byte-identical"
        ),
    }


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
    vanilla_root: Path,
    *,
    java_path: Path,
    verify_asset_objects: bool = True,
    supply_lock: Mapping[str, Any] | None = None,
) -> DistributionAudit:
    downloads = vanilla_root / "downloads" / MINECRAFT_VERSION
    metadata_path = downloads / f"{MINECRAFT_VERSION}.json"
    libraries_root = downloads / "libraries"
    assets_root = downloads / "assets"
    client_path = downloads / f"client-{MINECRAFT_VERSION}.jar"

    if not java_path.is_file() or not os.access(java_path, os.X_OK):
        raise BenchmarkFailure(f"Java executable is unavailable: {java_path}")

    if supply_lock is None:
        _, supply_lock = load_lock(MINECRAFT_VERSION)
    locked_metadata = supply_lock["metadata"]
    metadata_verified = validate_file(
        metadata_path,
        expected_sha1=locked_metadata["sha1"],
        expected_size=locked_metadata["size"],
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


def validate_server_jar(vanilla_root: Path) -> VerifiedFile:
    downloads = vanilla_root / "downloads" / MINECRAFT_VERSION
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
        # FramerateLimitTracker otherwise enters its 30 FPS SHORT_AFK state
        # after 60 seconds, even while the server-authoritative replay moves
        # the camera. MINIMIZED retains the iconified-window safety throttle.
        "inactivityFpsLimit": '"minimized"',
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
        "particles": "2",
        "mipmapLevels": "4",
        "cutoutLeaves": "true",
        "entityShadows": "true",
        # bbb renders the same six sorting targets through the official
        # `post_effect/transparency.json` graph, so the Vanilla side must use
        # the matching shader-transparency path as well.
        "improvedTransparency": "true",
        "weatherRadius": "10",
        "maxAnisotropyBit": "1",
        "textureFiltering": "1",
        "guiScale": "1",
        "forceUnicodeFont": "false",
        # Vanilla serializes the centered 70-degree FOV slider as 0.0.
        "fov": "0.0",
        "gamma": "0.5",
        "bobView": "true",
        # A headless compositor has no physical seat to focus either client.
        # Pin both sides to the same gameplay surface instead of allowing
        # Minecraft.pauseIfInactive to replace it with the pause screen.
        "pauseOnLostFocus": "false",
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


def write_vanilla_workload_debug_profile(game_dir: Path) -> Path:
    """Pins non-interactive Vanilla render workload counters for evidence runs."""
    game_dir.mkdir(parents=True, exist_ok=True)
    profile_path = game_dir / "debug-profile.json"
    profile_path.write_bytes(
        canonical_json_bytes(
            {
                "DataVersion": 4786,
                "custom": {
                    "minecraft:chunk_render_stats": "alwaysOn",
                    "minecraft:entity_render_stats": "alwaysOn",
                    "minecraft:particle_render_stats": "alwaysOn",
                    "minecraft:player_position": "alwaysOn",
                },
            }
        )
    )
    return profile_path


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
    prefer_wayland: bool = True,
) -> LaunchPlan:
    if not 1 <= server_port <= 65535:
        raise BenchmarkFailure(f"invalid server port: {server_port}")
    classpath = os.pathsep.join(
        [entry.path for entry in audit.libraries] + [audit.client.path]
    )
    jvm_argv = [
        str(java_path),
        "-Xms2G",
        "-Xmx4G",
        "-XX:+UseCompactObjectHeaders",
        "-XX:+AlwaysPreTouch",
        "-XX:+UseStringDeduplication",
        "-XX:+UseZGC",
        "-DMC_DEBUG_ENABLED",
    ]
    if prefer_wayland:
        jvm_argv.append("-DMC_DEBUG_PREFER_WAYLAND")
    jvm_argv.extend(
        [
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
        ]
    )
    argv = tuple(jvm_argv)
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
    vanilla_root: Path,
    server_port: int,
    control_port: int,
) -> tuple[str, ...]:
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise BenchmarkFailure(f"bbb-native executable is unavailable: {binary}")
    return (
        str(binary),
        "--vanilla-root",
        str(vanilla_root),
        "--server",
        f"127.0.0.1:{server_port}",
        "--username",
        BENCHMARK_USERNAME,
        "--connect-server",
        "--control",
        f"127.0.0.1:{control_port}",
        "--render-distance",
        str(VIEW_DISTANCE),
        "--cloud-range",
        "64",
        "--client-particles",
        "minimal",
        "--tutorial-step",
        "none",
        "--client-vsync",
        "false",
        "--client-framerate-limit",
        "260",
        "--inactivity-fps-limit",
        "minimized",
        "--client-gui-scale",
        "1",
        "--client-force-unicode-font",
        "false",
        "--client-fov",
        "70",
        "--client-gamma",
        "0.5",
        "--pause-on-lost-focus",
        "false",
        "--render-clouds",
        "fancy",
        "--cutout-leaves",
        "true",
        "--terrain-texture-filtering",
        "rgss",
        "--client-max-anisotropy-bit",
        "1",
        "--client-smooth-lighting",
        "true",
        "--client-view-bobbing",
        "true",
        "--improved-transparency",
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
    # Reach the reviewed camera pose four seconds before capture and then hold
    # it through the anchor. Both clients can finish their asynchronous chunk
    # compilation/occlusion publications without changing the measured motion
    # after the hold. The previous 24-tick pre-anchor settle admitted different
    # valid-but-stale BBB visibility graphs at the same screenshot tick.
    capture_hold = range(80, 169)
    capture_motion_tick = 120
    anchor_tick = 144
    marker_ticks = {
        0: "segment_start",
        anchor_tick - 1: "capture_start",
        anchor_tick: "anchor",
        segment_ticks - 1: "capture_end",
    }

    def motion_tick(offset: int) -> int:
        """Collapse settle holds without introducing a post-hold camera jump."""
        if offset < capture_hold.start:
            return round(offset * capture_motion_tick / capture_hold.start)
        if offset < capture_hold.stop:
            return capture_motion_tick
        return capture_motion_tick + offset - (capture_hold.stop - 1)

    final_motion_tick = motion_tick(segment_ticks - 1)

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
    for offset in range(segment_ticks):
        progress = motion_tick(offset) / final_motion_tick
        angle = math.radians((segment_ticks - 1) * 0.2 * progress)
        x = math.cos(angle) * 24.0
        z = math.sin(angle) * 24.0
        # Vanilla yaw 0 looks toward +Z and -90 toward +X. Point inward at
        # the controlled scene instead of away into naturally generated land.
        yaw = math.degrees(math.atan2(x, -z))
        # Keep the common replay pitch neutral. The benchmark is comparing
        # renderer work, not camera-sign parity, so a zero pitch prevents a
        # client-specific pitch convention from changing the visible set.
        orbit.append((x, 72.0, z, yaw, 0.0))
    append_segment("static_orbit", orbit)

    flight: list[tuple[float, float, float, float, float]] = []
    for offset in range(segment_ticks):
        progress = motion_tick(offset) / final_motion_tick
        x = -12.0 + 24.0 * progress
        z = 128.0 + math.sin(progress * math.tau * 2.0) * 2.0
        flight.append((x, 92.0, z, -90.0, 0.0))
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
    for offset in range(segment_ticks):
        index = min(
            motion_tick(offset) * len(boundary_positions) // (final_motion_tick + 1),
            len(boundary_positions) - 1,
        )
        x = boundary_positions[index]
        crossing.append((x, 88.0, -48.5, -90.0, 0.0))
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
        (
            "execute if score #run bbb_replay_control matches 1 "
            f"as @a[name={BENCHMARK_USERNAME},"
            f"scores={{bbb_replay=0..{final_tick}}}] "
            "run scoreboard players add @s bbb_replay 1"
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
        ("level-type", "minecraft:flat"),
        ("generator-settings", VOID_FLAT_GENERATOR_SETTINGS),
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
        # Static-orbit scene: opaque floor, a bounded translucent volume, and
        # an open roof so both clients see the same controlled contents.
        "fill -40 63 -40 40 63 40 minecraft:polished_andesite",
        "fill -24 64 -24 24 64 24 minecraft:water",
        "fill -24 65 -24 24 76 -24 minecraft:glass",
        "fill -24 65 24 24 76 24 minecraft:glass",
        "fill -24 65 -23 -24 76 23 minecraft:glass",
        "fill 24 65 -23 24 76 23 minecraft:glass",
        # Preloaded-flight scene: a bounded arena keeps every drawn landmark
        # within the common near field while the camera changes its frustum.
        "fill -32 79 120 32 79 136 minecraft:stone_bricks",
        "fill -32 80 120 -32 112 136 minecraft:polished_andesite",
        "fill 32 80 120 32 112 136 minecraft:polished_andesite",
        # Chunk-boundary scene: opaque partitions bound each cell's visible
        # contents while the authoritative camera still crosses chunk centers.
        "fill -16 79 -64 160 79 -32 minecraft:deepslate_tiles",
    ]
    pillar_blocks = (
        "minecraft:polished_andesite",
        "minecraft:oak_leaves[persistent=true]",
        "minecraft:blue_stained_glass",
        "minecraft:glowstone",
    )
    for x_index, x in enumerate(range(-24, 25, 8)):
        for z_index, z in enumerate((122, 128, 134)):
            height = 4 + ((x_index + z_index) % 4) * 3
            block = pillar_blocks[(x_index + z_index) % len(pillar_blocks)]
            commands.append(
                f"fill {x - 2} 80 {z - 2} {x + 2} {80 + height} {z + 2} {block}"
            )
    for wall_x in range(8, 161, 16):
        commands.append(
            f"fill {wall_x} 80 -64 {wall_x} 104 -32 "
            "minecraft:polished_andesite"
        )
    for x_index, x in enumerate(range(0, 160, 16)):
        height = 5 + (x_index % 5) * 2
        block = pillar_blocks[(x_index + 1) % len(pillar_blocks)]
        commands.append(
            f"fill {x + 4} 80 -60 {x + 10} {80 + height} -54 {block}"
        )
        commands.append(
            f"fill {x + 4} 80 -42 {x + 10} {84 + height} -36 "
            "minecraft:glass hollow"
        )
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
    for index, x in enumerate(range(-8, 25, 16)):
        z = 128 + (-4 if index % 2 == 0 else 4)
        commands.append(
            "summon minecraft:armor_stand "
            f"{x} 90 {z} "
            "{Tags:[\"bbb_benchmark_scene\",\"bbb_benchmark_armor\"],NoGravity:1b,"
            "Invulnerable:1b,PersistenceRequired:1b,"
            f"Rotation:[{(index * 36) % 360:.1f}f,0.0f]}}"
        )
    for index, wall_x in enumerate(range(24, 153, 32)):
        commands.append(
            "summon minecraft:armor_stand "
            f"{wall_x - 4} 86 -48 "
            "{Tags:[\"bbb_benchmark_scene\",\"bbb_benchmark_armor\"],NoGravity:1b,"
            "Invulnerable:1b,PersistenceRequired:1b,"
            f"Rotation:[{(index * 45) % 360:.1f}f,0.0f]}}"
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
    process: subprocess.Popen[Any] | None,
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
        if process is not None:
            status = process.poll()
            if status is not None:
                raise BenchmarkFailure(
                    f"process exited with status {status} before log marker "
                    f"{marker!r}; see {path}"
                )
        time.sleep(0.25)
    raise BenchmarkFailure(f"timed out waiting for log marker {marker!r}: {path}")


def replay_capture_markers(
    replay: Mapping[str, Any],
) -> list[Mapping[str, Any]]:
    frames = replay.get("frames")
    if not isinstance(frames, list):
        raise BenchmarkFailure("replay lacks frames for capture markers")
    markers = [
        frame
        for frame in frames
        if isinstance(frame, Mapping)
        and frame.get("marker") in ("capture_start", "anchor", "capture_end")
    ]
    expected = ["capture_start", "anchor", "capture_end"] * 3
    if [frame.get("marker") for frame in markers] != expected:
        raise BenchmarkFailure("replay capture markers are incomplete or out of order")
    return markers


def benchmark_marker_line(replay_hash: str, frame: Mapping[str, Any]) -> str:
    return (
        f"BBB_BENCHMARK_MARKER {replay_hash} "
        f"{frame['capture_segment']} {frame['marker']} {frame['server_tick']}"
    )


def marker_timeline_observation(
    frame: Mapping[str, Any], *, elapsed_seconds: float
) -> dict[str, Any]:
    return {
        "capture_segment": str(frame["capture_segment"]),
        "marker": str(frame["marker"]),
        "server_tick": int(frame["server_tick"]),
        "elapsed_seconds": max(0.0, elapsed_seconds),
    }


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
    vanilla_root: Path,
    java_path: Path,
    artifact_dir: Path,
    server_port: int,
    accept_eula: bool,
    timeout: float,
) -> WorldSnapshot:
    server = validate_server_jar(vanilla_root)
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


def parse_mangohud_csv(path: Path) -> dict[str, Any]:
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
    system: dict[str, str] = {}
    if header_index >= 2:
        system_reader = csv.DictReader(lines[:2])
        system_row = next(system_reader, None)
        if system_row is not None:
            system = {
                str(key).strip(): str(value).strip()
                for key, value in system_row.items()
                if key is not None and value is not None
            }
    reader = csv.DictReader(lines[header_index:])
    frames: list[float] = []
    gpu_load: list[float] = []
    gpu_clock: list[float] = []
    gpu_temp: list[float] = []
    cpu_temp: list[float] = []
    elapsed_seconds: list[float] = []
    samples: list[dict[str, float]] = []
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
        sample = {"frame_ms": frame_ms}
        for name, output, aliases in (
            ("gpu_load", gpu_load, ("gpu_load", "gpu load")),
            (
                "gpu_clock_mhz",
                gpu_clock,
                ("gpu_core_clock", "gpu_clock", "gpu clock"),
            ),
            (
                "gpu_temp_c",
                gpu_temp,
                ("gpu_temp", "gpu_temperature", "gpu temp"),
            ),
            (
                "cpu_temp_c",
                cpu_temp,
                ("cpu_temp", "cpu_temperature", "cpu temp"),
            ),
        ):
            value = _float_field(row, aliases)
            if value is not None and (
                not name.endswith("_temp_c") or value > 0.0
            ):
                output.append(value)
                sample[name] = value
        elapsed_ns = _float_field(row, ("elapsed",))
        if elapsed_ns is not None:
            if elapsed_ns < 0.0:
                raise BenchmarkFailure("MangoHud elapsed time must not be negative")
            elapsed = elapsed_ns / 1_000_000_000.0
            elapsed_seconds.append(elapsed)
            sample["elapsed_seconds"] = elapsed
        samples.append(sample)
    if not frames:
        raise BenchmarkFailure("MangoHud log contains no frame samples")
    if elapsed_seconds and len(elapsed_seconds) != len(frames):
        raise BenchmarkFailure("MangoHud elapsed time is missing from some rows")
    if any(
        right <= left
        for left, right in zip(elapsed_seconds, elapsed_seconds[1:])
    ):
        raise BenchmarkFailure("MangoHud elapsed time must increase monotonically")
    return {
        "frame_ms": frames,
        "gpu_load": gpu_load,
        "gpu_clock_mhz": gpu_clock,
        "gpu_temp_c": gpu_temp,
        "cpu_temp_c": cpu_temp,
        "elapsed_seconds": elapsed_seconds,
        "samples": samples,
        "system": system,
    }


def split_mangohud_capture_by_replay(
    capture: Mapping[str, Sequence[Any]],
    replay: Mapping[str, Any],
    marker_timeline: Sequence[Mapping[str, Any]],
) -> dict[str, dict[str, list[float]]]:
    samples = capture.get("samples", ())
    if not samples or any(
        not isinstance(sample, Mapping) or "elapsed_seconds" not in sample
        for sample in samples
    ):
        raise BenchmarkFailure(
            "MangoHud segment summaries require elapsed time for every sample"
        )
    frames = replay.get("frames")
    if not isinstance(frames, list):
        raise BenchmarkFailure("replay lacks frames for MangoHud segmentation")
    expected_timeline = [
        (
            str(frame["capture_segment"]),
            str(frame["marker"]),
            int(frame["server_tick"]),
        )
        for frame in replay_capture_markers(replay)
    ]
    observed_timeline: list[tuple[str, str, int]] = []
    elapsed_by_boundary: dict[tuple[str, str], float] = {}
    previous_elapsed = -1.0
    for observation in marker_timeline:
        marker = str(observation.get("marker", ""))
        observed = (
            str(observation.get("capture_segment", "")),
            marker,
            int(observation.get("server_tick", -1)),
        )
        elapsed = float(observation.get("elapsed_seconds", -1.0))
        if elapsed < 0.0 or elapsed < previous_elapsed:
            raise BenchmarkFailure(
                "capture marker elapsed times must be non-negative and monotonic"
        )
        previous_elapsed = elapsed
        observed_timeline.append(observed)
        if marker in ("capture_start", "capture_end"):
            elapsed_by_boundary[(observed[0], observed[1])] = elapsed
    if observed_timeline != expected_timeline:
        raise BenchmarkFailure(
            "capture marker timeline does not match replay marker order"
        )
    starts = {
        segment: elapsed
        for (segment, marker), elapsed in elapsed_by_boundary.items()
        if marker == "capture_start"
    }
    ends = {
        segment: elapsed
        for (segment, marker), elapsed in elapsed_by_boundary.items()
        if marker == "capture_end"
    }
    if set(starts) != set(ends) or len(starts) != 3:
        raise BenchmarkFailure("replay capture segment boundaries are incomplete")
    segmented: dict[str, dict[str, list[float]]] = {}
    for segment in starts:
        start_seconds = starts[segment]
        end_seconds = ends[segment]
        if end_seconds <= start_seconds:
            raise BenchmarkFailure(
                f"capture marker timeline has an empty segment: {segment}"
            )
        selected = [
            sample
            for sample in samples
            if start_seconds
            <= float(sample["elapsed_seconds"])
            < end_seconds
        ]
        if not selected:
            raise BenchmarkFailure(f"MangoHud segment has no samples: {segment}")
        segmented[segment] = {
            name: [
                float(sample[name])
                for sample in selected
                if name in sample
            ]
            for name in (
                "frame_ms",
                "gpu_load",
                "gpu_clock_mhz",
                "gpu_temp_c",
                "cpu_temp_c",
            )
        }
    return segmented


def summarize_mangohud_logs(
    paths: Sequence[Path],
    replay: Mapping[str, Any],
    marker_timeline: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    if len(paths) != 1:
        raise BenchmarkFailure(
            "formal MangoHud capture requires exactly one raw frame log"
        )
    capture = parse_mangohud_csv(paths[0])
    segments = split_mangohud_capture_by_replay(
        capture, replay, marker_timeline
    )
    return {
        "overall": [asdict(summarize_frames(capture))],
        "segments": {
            segment: asdict(summarize_frames(values))
            for segment, values in segments.items()
        },
        "presented_frames": len(capture["frame_ms"]),
    }


def append_formal_segment_samples(
    aggregate: dict[str, dict[str, list[float]]],
    split: Mapping[str, Mapping[str, Sequence[float]]],
    metric_names: Sequence[str],
) -> None:
    """Append only replay capture segments to formal frame distributions.

    MangoHud logging spans the server's step/hold transitions between capture
    segments. Those frames remain in the raw artifact and in the perf process
    totals, but they are not part of any frozen visible workload and therefore
    must not influence formal frame percentiles or hitch counts.
    """
    for segment in FORMAL_CAPTURE_SEGMENTS:
        values = split.get(segment)
        if not isinstance(values, Mapping):
            raise BenchmarkFailure(
                f"formal frame capture lacks segment {segment}"
            )
        for name in metric_names:
            samples = values.get(name, ())
            aggregate[segment][name].extend(samples)
            aggregate["overall"][name].extend(samples)


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

    def optional_max(name: str) -> float | None:
        values = list(capture.get(name, ()))
        return max(values) if values else None

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
        gpu_temp_peak_c=optional_max("gpu_temp_c"),
        cpu_temp_mean_c=optional_mean("cpu_temp_c"),
        cpu_temp_peak_c=optional_max("cpu_temp_c"),
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


def build_perf_stat_argv(
    *,
    perf_path: Path,
    target_pid: int,
    output_path: Path,
) -> tuple[str, ...]:
    if not perf_path.is_file() or not os.access(perf_path, os.X_OK):
        raise BenchmarkFailure(f"perf executable is unavailable: {perf_path}")
    if target_pid <= 0:
        raise BenchmarkFailure("perf target PID must be positive")
    return (
        str(perf_path),
        "stat",
        "-x,",
        "-o",
        str(output_path),
        "-p",
        str(target_pid),
        "-e",
        ",".join(REQUIRED_PERF_EVENTS),
    )


class PerfStatSession:
    def __init__(
        self,
        *,
        perf_path: Path,
        target_pid: int,
        output_path: Path,
    ) -> None:
        self.argv = build_perf_stat_argv(
            perf_path=perf_path,
            target_pid=target_pid,
            output_path=output_path,
        )
        self.output_path = output_path
        self.managed: ManagedProcess | None = None
        self.process: subprocess.Popen[Any] | None = None

    def start(self) -> dict[str, Any]:
        if self.process is not None:
            raise BenchmarkFailure("perf stat session was already started")
        self.output_path.parent.mkdir(parents=True, exist_ok=True)
        self.managed = ManagedProcess(
            self.argv,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
        ).start()
        self.process = self.managed.process
        assert self.process is not None
        time.sleep(0.1)
        if self.process.poll() is not None:
            stderr = self.process.stderr.read() if self.process.stderr else ""
            raise BenchmarkFailure(
                "perf stat failed to attach: "
                + (stderr.strip() or f"exit {self.process.returncode}")
            )
        return {
            "argv": list(self.argv),
            "target_pid": int(self.argv[-3]),
            "output": str(self.output_path),
        }

    def stop(self, timeout: float = 10.0) -> dict[str, Any]:
        process = self.process
        if process is None:
            raise BenchmarkFailure("perf stat session was not started")
        if process.poll() is None:
            try:
                assert self.managed is not None
                self.managed.stop(signal.SIGINT, grace_seconds=timeout)
            except ProcessFailure as error:
                raise BenchmarkFailure(str(error)) from error
        stderr = process.stderr.read() if process.stderr else ""
        if process.returncode not in (0, 130, -signal.SIGINT):
            raise BenchmarkFailure(
                "perf stat capture failed: "
                + (stderr.strip() or f"exit {process.returncode}")
            )
        if not self.output_path.is_file() or self.output_path.stat().st_size <= 0:
            raise BenchmarkFailure("perf stat did not produce an output file")
        return {
            "exit_status": process.returncode,
            "output": str(self.output_path),
            "bytes": self.output_path.stat().st_size,
            "sha256": file_digest(self.output_path, "sha256"),
        }

    def cleanup(self) -> None:
        if self.process is not None and self.process.poll() is None:
            self.stop()


def parse_weston_timeline(
    path: Path,
    *,
    surface_title: str,
    start_monotonic: float | None = None,
    end_monotonic: float | None = None,
) -> dict[str, Any]:
    surface_ids: set[int] = set()
    commits: list[float] = []
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise BenchmarkFailure(f"cannot read Weston timeline {path}: {error}") from error
    decoded: list[Mapping[str, Any]] = []
    for line in lines:
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError as error:
            raise BenchmarkFailure(f"invalid Weston timeline event: {line}") from error
        if isinstance(event, Mapping):
            decoded.append(event)
            if (
                event.get("type") == "weston_surface"
                and surface_title in str(event.get("desc", ""))
            ):
                surface_ids.add(int(event["id"]))
    if len(surface_ids) != 1:
        raise BenchmarkFailure(
            f"Weston timeline expected one {surface_title!r} surface, "
            f"found {len(surface_ids)}"
        )
    surface_id = next(iter(surface_ids))
    for event in decoded:
        if (
            event.get("N") == "core_commit_damage"
            and event.get("ws") == surface_id
        ):
            timestamp = event.get("T")
            if (
                not isinstance(timestamp, list)
                or len(timestamp) != 2
            ):
                raise BenchmarkFailure("Weston timeline event has invalid time")
            commits.append(float(timestamp[0]) + float(timestamp[1]) / 1e9)
    if start_monotonic is not None:
        commits = [
            timestamp
            for timestamp in commits
            if timestamp >= start_monotonic
        ]
    if end_monotonic is not None:
        commits = [
            timestamp
            for timestamp in commits
            if timestamp <= end_monotonic
        ]
    intervals = [
        (right - left) * 1000.0
        for left, right in zip(commits, commits[1:])
        if right > left
    ]
    if not intervals:
        raise BenchmarkFailure("Weston timeline has no surface commit intervals")
    return {
        "surface_title": surface_title,
        "surface_id": surface_id,
        "surface_commits": len(commits),
        "commit_interval": asdict(
            summarize_frames({"frame_ms": intervals})
        ),
        "scope": (
            "Weston/Xwayland compositor-observed surface commit intervals; "
            "not client CPU duration, presentation completion, or GPU duration"
        ),
    }


class WestonTimelineSession:
    def __init__(
        self,
        *,
        weston_debug_path: Path,
        wayland_display: str,
        output_path: Path,
        surface_title: str,
    ) -> None:
        if not weston_debug_path.is_file() or not os.access(
            weston_debug_path, os.X_OK
        ):
            raise BenchmarkFailure(
                f"weston-debug executable is unavailable: {weston_debug_path}"
            )
        self.argv = (str(weston_debug_path), "timeline")
        self.output_path = output_path
        self.surface_title = surface_title
        self.environment = {
            **os.environ,
            "XDG_RUNTIME_DIR": os.environ.get(
                "XDG_RUNTIME_DIR", "/run/user/1000"
            ),
            "WAYLAND_DISPLAY": wayland_display,
        }
        self.process: subprocess.Popen[Any] | None = None
        self.managed: ManagedProcess | None = None
        self.output: Any = None
        self.start_monotonic: float | None = None

    def start(self) -> dict[str, Any]:
        if self.process is not None:
            raise BenchmarkFailure("Weston timeline session was already started")
        self.output_path.parent.mkdir(parents=True, exist_ok=True)
        self.output = self.output_path.open("w", encoding="utf-8")
        self.managed = ManagedProcess(
            self.argv,
            stdin=subprocess.DEVNULL,
            stdout=self.output,
            stderr=subprocess.STDOUT,
            env=self.environment,
        ).start()
        self.process = self.managed.process
        assert self.process is not None
        time.sleep(0.1)
        if self.process.poll() is not None:
            self.output.close()
            raise BenchmarkFailure("weston-debug timeline failed to start")
        self.start_monotonic = time.monotonic()
        return {
            "argv": list(self.argv),
            "output": str(self.output_path),
            "surface_title": self.surface_title,
            "wayland_display": self.environment["WAYLAND_DISPLAY"],
        }

    def stop(self, timeout: float = 5.0) -> dict[str, Any]:
        process = self.process
        if process is None:
            raise BenchmarkFailure("Weston timeline session was not started")
        if process.poll() is None:
            end_monotonic = time.monotonic()
            try:
                assert self.managed is not None
                self.managed.terminate(grace_seconds=timeout)
            except ProcessFailure as error:
                raise BenchmarkFailure(str(error)) from error
        else:
            end_monotonic = time.monotonic()
        if self.output is not None and not self.output.closed:
            self.output.close()
        parsed = parse_weston_timeline(
            self.output_path,
            surface_title=self.surface_title,
            start_monotonic=self.start_monotonic,
            end_monotonic=end_monotonic,
        )
        return {
            "exit_status": process.returncode,
            "output": str(self.output_path),
            "bytes": self.output_path.stat().st_size,
            "sha256": file_digest(self.output_path, "sha256"),
            "start_monotonic": self.start_monotonic,
            "end_monotonic": end_monotonic,
            **parsed,
        }


class ProcessGroup:
    def __init__(self, argv: Sequence[str], **popen_options: Any) -> None:
        self.argv = tuple(argv)
        self.popen_options = popen_options
        self.managed: ManagedProcess | None = None
        self.process: subprocess.Popen[Any] | None = None

    def start(self) -> subprocess.Popen[Any]:
        if self.process is not None:
            raise BenchmarkFailure("process group was already started")
        self.managed = ManagedProcess(self.argv, **self.popen_options).start()
        self.process = self.managed.process
        assert self.process is not None
        return self.process

    def cleanup(self, timeout: float = 5.0) -> None:
        if self.managed is None:
            return
        try:
            self.managed.terminate(grace_seconds=timeout)
        except ProcessFailure as error:
            raise BenchmarkFailure(str(error)) from error

    def kill(self, timeout: float = 5.0) -> None:
        if self.managed is None:
            return
        try:
            self.managed.kill(grace_seconds=timeout)
        except ProcessFailure as error:
            raise BenchmarkFailure(str(error)) from error

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
    gpu_duration_blockers: list[dict[str, str]] = []
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
        candidates = [root / "events" / group / name / "id" for root in trace_roots]
        trace_events[event] = {
            "paths": [str(path) for path in candidates],
            "present": any(path.is_file() for path in candidates),
            "readable": any(os.access(path, os.R_OK) for path in candidates),
        }
    missing_events = sorted(
        event for event, status in trace_events.items() if not status["present"]
    )
    if missing_events:
        gpu_duration_blockers.append(
            {
                "code": "trace_events_absent",
                "detail": f"this kernel exposes no {', '.join(missing_events)}",
            }
        )
    # Probe the capability directly instead of inferring it from permission bits:
    # perf resolves the tracepoint ids out of tracefs and then opens them, and
    # only an actual open proves an unprivileged capture can run.
    gpu_chain_events = ",".join(GPU_DURATION_CHAIN_EVENTS)
    gpu_probe_argv = (
        "perf",
        "record",
        "-q",
        "-o",
        "/dev/null",
        "-e",
        gpu_chain_events,
        "-a",
        "--",
        "/bin/true",
    )
    try:
        gpu_probe = subprocess.run(
            gpu_probe_argv,
            check=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=30.0,
        )
        gpu_probe_status = {
            "argv": list(gpu_probe_argv),
            "returncode": gpu_probe.returncode,
            "output": gpu_probe.stdout,
        }
    except (OSError, subprocess.TimeoutExpired) as error:
        gpu_probe_status = {
            "argv": list(gpu_probe_argv),
            "returncode": None,
            "output": f"{type(error).__name__}: {error}",
        }
    if gpu_probe_status["returncode"] != 0 or "can't access trace events" in (
        gpu_probe_status["output"] or ""
    ):
        gpu_duration_blockers.append(
            {
                "code": "unprivileged_tracepoint_capture_unavailable",
                "detail": (
                    "perf cannot open the GPU chain tracepoints as this user; "
                    "provision the runner with tracefs group read access and "
                    "kernel.perf_event_paranoid=-1"
                ),
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
        "gpu_duration_chain": list(GPU_DURATION_CHAIN_EVENTS),
        "gpu_duration_probe": gpu_probe_status,
        "gpu_duration_blockers": gpu_duration_blockers,
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
        "gpu_duration_preflight_ready": not gpu_duration_blockers,
        # A per-client GPU execution duration may be published once the chain can
        # actually be captured. The claim still rests on the capture's own
        # association ratio, which `gpu_duration_capture.summarize` reports per run.
        "gpu_duration_claim_allowed": not gpu_duration_blockers,
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
                "code": "mangohud_teardown_unstable",
                "detail": (
                    "MangoHud 0.8.4 capture closes successfully, but both clients "
                    "require post-capture SIGKILL to avoid reproducible teardown crashes"
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


def summarize_workload_evidence(
    *,
    vanilla: Mapping[str, Any],
    bbb: Mapping[str, Any],
    vanilla_calibration: Mapping[str, Any],
    bbb_calibration: Mapping[str, Any],
    bbb_provenance: Mapping[str, Any],
    vanilla_debug: Mapping[str, Any] | None = None,
    visual_review_status: str,
    visual_review_note: str,
    visual_artifact_dir: Path,
) -> dict[str, Any]:
    if visual_review_status not in ("match", "mismatch"):
        raise BenchmarkFailure(
            f"invalid visual workload review status: {visual_review_status}"
        )
    if not visual_review_note.strip():
        raise BenchmarkFailure("visual workload review note must not be empty")
    for name, smoke, expected_client in (
        ("vanilla", vanilla, "vanilla"),
        ("bbb", bbb, "bbb"),
    ):
        if smoke.get("client") != expected_client:
            raise BenchmarkFailure(f"{name} smoke client identity mismatch")
        if smoke.get("status") != "passed":
            raise BenchmarkFailure(f"{name} workload smoke did not pass")
        if smoke.get("position_corrections") != 0:
            raise BenchmarkFailure(f"{name} workload observed a position correction")
    if vanilla.get("workload_debug_profile") is not False:
        raise BenchmarkFailure(
            "Vanilla parity anchors must disable the debug profile so HUD workload matches bbb"
        )

    launch_manifest = load_json_object(
        Path(str(bbb.get("launch_manifest"))), "bbb workload launch manifest"
    )
    capture_binary = launch_manifest.get("binary")
    if not isinstance(capture_binary, Mapping) or not isinstance(
        capture_binary.get("sha256"), str
    ):
        raise BenchmarkFailure("bbb workload launch manifest lacks binary identity")
    validated_provenance = validate_bbb_binary_provenance(
        bbb_provenance,
        expected_capture_commit=None,
        expected_capture_sha256=str(capture_binary["sha256"]),
    )

    contract_keys = (
        "replay_hash",
        "world_hash",
        "resource_hash",
        "settings_hash",
        "scene_contract_hash",
    )
    contracts: dict[str, Any] = {}
    for key in contract_keys:
        value = vanilla.get(key)
        if value is None or value != bbb.get(key):
            raise BenchmarkFailure(f"workload contract {key} mismatch")
        contracts[key] = value
    calibration_evidence = {}
    for client, calibration in (
        ("vanilla", vanilla_calibration),
        ("bbb", bbb_calibration),
    ):
        if (
            calibration.get("client") != client
            or calibration.get("status") != "calibrated"
            or calibration.get("contracts") != contracts
        ):
            raise BenchmarkFailure(
                f"{client} same-client visual calibration is invalid"
            )
        pixel_calibration = calibration.get("pixel_evidence")
        if not isinstance(pixel_calibration, Mapping) or pixel_calibration.get(
            "status"
        ) != "passed":
            raise BenchmarkFailure(
                f"{client} same-client visual calibration did not pass"
            )
        calibration_evidence[client] = {
            "visual_calibration_hash": calibration.get(
                "visual_calibration_hash"
            ),
            "client_identity": calibration.get("client_identity"),
            "observed_maxima": calibration.get("observed_maxima"),
        }
    debug_evidence = None
    if vanilla_debug is not None:
        if (
            vanilla_debug.get("client") != "vanilla"
            or vanilla_debug.get("status") != "passed"
            or vanilla_debug.get("workload_debug_profile") is not True
        ):
            raise BenchmarkFailure("Vanilla debug workload evidence is invalid")
        for key, value in contracts.items():
            if vanilla_debug.get(key) != value:
                raise BenchmarkFailure(
                    f"Vanilla debug workload contract {key} mismatch"
                )
        debug_evidence = {
            "scope": (
                "supplementary Vanilla-only chunk/entity/particle/player counters; "
                "excluded from parity screenshots and formal timing"
            ),
            "screenshots": vanilla_debug.get("screenshots"),
            "server_camera_log": vanilla_debug.get("server_camera_log"),
        }
    vanilla_camera = vanilla.get("server_camera_log")
    bbb_camera = bbb.get("server_camera_log")
    if not isinstance(vanilla_camera, Mapping) or not isinstance(
        bbb_camera, Mapping
    ):
        raise BenchmarkFailure("workload smoke lacks a server camera log")
    if (
        vanilla_camera.get("sha256") != bbb_camera.get("sha256")
        or vanilla_camera.get("markers") != bbb_camera.get("markers")
    ):
        raise BenchmarkFailure("server camera logs differ")

    segments = (
        "static_orbit",
        "preloaded_flight",
        "chunk_boundary_crossing",
    )
    screenshots: dict[str, dict[str, Any]] = {}
    for client, smoke in (("vanilla", vanilla), ("bbb", bbb)):
        by_segment = {
            screenshot.get("capture_segment"): screenshot
            for screenshot in smoke.get("screenshots", ())
            if isinstance(screenshot, Mapping)
        }
        if set(by_segment) != set(segments):
            raise BenchmarkFailure(
                f"{client} workload screenshots do not cover all segments"
            )
        for segment in segments:
            screenshot = by_segment[segment]
            if not screenshot.get("sha256") or not screenshot.get("path"):
                raise BenchmarkFailure(
                    f"{client} {segment} screenshot lacks content evidence"
                )
            screenshots.setdefault(segment, {})[client] = dict(screenshot)
    for segment, pair in screenshots.items():
        if pair["vanilla"].get("server_tick") != pair["bbb"].get("server_tick"):
            raise BenchmarkFailure(f"{segment} screenshot tick mismatch")
    pixel_evidence = analyze_visual_anchor_pairs(screenshots, visual_artifact_dir)

    observations = {
        observation.get("capture_segment"): observation
        for observation in bbb.get("anchor_observations", ())
        if isinstance(observation, Mapping)
    }
    if set(observations) != set(segments):
        raise BenchmarkFailure("bbb anchor observations do not cover all segments")
    required_draws = {
        "static_orbit": ("opaque_draw_calls", "translucent_draw_calls"),
        "preloaded_flight": (
            "opaque_draw_calls",
            "cutout_draw_calls",
            "translucent_draw_calls",
            "entity_scene_draw_calls",
        ),
        "chunk_boundary_crossing": (
            "opaque_draw_calls",
            "translucent_draw_calls",
            "entity_scene_draw_calls",
        ),
    }
    bbb_render_evidence: dict[str, Any] = {}
    selected_fields = (
        "terrain_selected_chunks",
        "terrain_resident_chunks",
        "terrain_selected_sections",
        "terrain_resident_sections",
        "terrain_pending_jobs",
        "terrain_cache_ready",
        "visible_sections",
        "resident_bytes",
        "opaque_draw_calls",
        "cutout_draw_calls",
        "translucent_draw_calls",
        "opaque_faces",
        "cutout_faces",
        "translucent_faces",
        "entity_render_candidates",
        "entity_render_projected",
        "entity_scene_draw_calls",
        "active_particle_instances",
        "particle_draw_calls",
        "draw_calls",
    )
    for segment in segments:
        renderer = observations[segment].get("renderer")
        if not isinstance(renderer, Mapping):
            raise BenchmarkFailure(f"bbb {segment} lacks renderer counters")
        for field in required_draws[segment]:
            if int(renderer.get(field, 0)) <= 0:
                raise BenchmarkFailure(f"bbb {segment} lacks required {field}")
        if int(renderer.get("visible_sections", 0)) <= 0:
            raise BenchmarkFailure(f"bbb {segment} has no visible sections")
        require_bbb_terrain_residency(
            renderer, capture_segment=segment
        )
        if int(renderer.get("active_particle_instances", -1)) != 0:
            raise BenchmarkFailure(
                f"bbb {segment} minimal-particle workload is not stable"
            )
        bbb_render_evidence[segment] = {
            field: renderer.get(field) for field in selected_fields
        }

    evidence_hash = canonical_sha256(
        {
            "contracts": contracts,
            "server_camera_log": {
                "sha256": vanilla_camera.get("sha256"),
                "markers": vanilla_camera.get("markers"),
            },
            "anchor_ticks": {
                segment: screenshots[segment]["vanilla"].get("server_tick")
                for segment in segments
            },
        }
    )
    visual_gate_passed = (
        visual_review_status == "match" and pixel_evidence["status"] == "passed"
    )
    return {
        "schema": BENCHMARK_SCHEMA,
        "bbb_commit": validated_provenance["evidence_commit"],
        "bbb_capture_commit": validated_provenance["capture_commit"],
        "bbb_binary_sha256": validated_provenance["binary_sha256"],
        "bbb_provenance": validated_provenance,
        "status": "matched" if visual_gate_passed else "mismatched",
        "formal_timing_sample": False,
        "workload_evidence_hash": evidence_hash,
        "contracts": contracts,
        "server_camera_log": {
            "sha256": vanilla_camera.get("sha256"),
            "markers": vanilla_camera.get("markers"),
        },
        "visual_review": {
            "status": visual_review_status,
            "note": visual_review_note,
        },
        "pixel_evidence": pixel_evidence,
        "same_client_calibration": calibration_evidence,
        "screenshot_pairs": [
            {
                "capture_segment": segment,
                **screenshots[segment],
            }
            for segment in segments
        ],
        "bbb_renderer_counters": bbb_render_evidence,
        "vanilla_debug_evidence": debug_evidence,
        "counter_scope": (
            "bbb counters are renderer-native evidence; optional Vanilla-only "
            "debug counters are supplementary and are not treated as numerically "
            "identical counter semantics"
        ),
    }


def _visual_calibration_client_identity(
    smoke: Mapping[str, Any], client: str
) -> dict[str, Any]:
    launch_path = Path(str(smoke.get("launch_manifest", "")))
    launch = load_json_object(launch_path, f"{client} launch manifest")
    if client == "bbb":
        binary = launch.get("binary")
        if not isinstance(binary, Mapping):
            raise BenchmarkFailure("bbb launch manifest lacks binary identity")
        identity = {
            "sha256": binary.get("sha256"),
            "size": binary.get("size"),
        }
    elif client == "vanilla":
        audit = launch.get("audit")
        official = audit.get("client") if isinstance(audit, Mapping) else None
        if not isinstance(official, Mapping):
            raise BenchmarkFailure("Vanilla launch manifest lacks client identity")
        identity = {
            "sha1": official.get("sha1"),
            "size": official.get("size"),
            "options_hash": launch.get("options_hash"),
        }
    else:
        raise BenchmarkFailure(f"invalid visual calibration client: {client}")
    if any(value is None for value in identity.values()):
        raise BenchmarkFailure(f"{client} launch identity is incomplete")
    return identity


def summarize_visual_calibration(
    *,
    baseline: Mapping[str, Any],
    repeat: Mapping[str, Any],
    client: str,
    visual_artifact_dir: Path,
) -> dict[str, Any]:
    for label, smoke in (("baseline", baseline), ("repeat", repeat)):
        if smoke.get("client") != client:
            raise BenchmarkFailure(f"{label} visual calibration client mismatch")
        if smoke.get("status") != "passed":
            raise BenchmarkFailure(f"{label} visual calibration smoke did not pass")
        if smoke.get("position_corrections") != 0:
            raise BenchmarkFailure(
                f"{label} visual calibration observed a position correction"
            )
        if client == "vanilla" and smoke.get("workload_debug_profile") is not False:
            raise BenchmarkFailure(
                "Vanilla visual calibration must disable the debug profile"
            )

    contract_keys = (
        "replay_hash",
        "world_hash",
        "resource_hash",
        "settings_hash",
        "scene_contract_hash",
    )
    contracts = {}
    for key in contract_keys:
        value = baseline.get(key)
        if value is None or value != repeat.get(key):
            raise BenchmarkFailure(f"visual calibration {key} mismatch")
        contracts[key] = value

    baseline_identity = _visual_calibration_client_identity(baseline, client)
    repeat_identity = _visual_calibration_client_identity(repeat, client)
    if baseline_identity != repeat_identity:
        raise BenchmarkFailure("visual calibration client binary identity mismatch")

    baseline_camera = baseline.get("server_camera_log")
    repeat_camera = repeat.get("server_camera_log")
    if not isinstance(baseline_camera, Mapping) or not isinstance(
        repeat_camera, Mapping
    ):
        raise BenchmarkFailure("visual calibration lacks a server camera log")
    if (
        baseline_camera.get("sha256") != repeat_camera.get("sha256")
        or baseline_camera.get("markers") != repeat_camera.get("markers")
    ):
        raise BenchmarkFailure("visual calibration server camera logs differ")

    segments = tuple(VISUAL_ROIS)
    screenshots: dict[str, dict[str, Any]] = {}
    for label, smoke in (("vanilla", baseline), ("bbb", repeat)):
        by_segment = {
            screenshot.get("capture_segment"): screenshot
            for screenshot in smoke.get("screenshots", ())
            if isinstance(screenshot, Mapping)
        }
        if set(by_segment) != set(segments):
            raise BenchmarkFailure(
                f"{label} visual calibration screenshots do not cover all segments"
            )
        for segment in segments:
            screenshot = by_segment[segment]
            if not screenshot.get("sha256") or not screenshot.get("path"):
                raise BenchmarkFailure(
                    f"{label} visual calibration {segment} lacks content evidence"
                )
            screenshots.setdefault(segment, {})[label] = dict(screenshot)
    for segment, pair in screenshots.items():
        if pair["vanilla"].get("server_tick") != pair["bbb"].get("server_tick"):
            raise BenchmarkFailure(f"visual calibration {segment} tick mismatch")

    pixel_evidence = analyze_visual_anchor_pairs(
        screenshots, visual_artifact_dir
    )
    fields = (
        "raw_mae",
        *(threshold.removeprefix("max_") for threshold in VISUAL_PIXEL_THRESHOLDS),
    )
    observed_maxima = {
        field: max(float(region[field]) for region in pixel_evidence["regions"])
        for field in fields
    }
    evidence_hash = canonical_sha256(
        {
            "client": client,
            "contracts": contracts,
            "client_identity": baseline_identity,
            "server_camera_log": {
                "sha256": baseline_camera.get("sha256"),
                "markers": baseline_camera.get("markers"),
            },
            "screenshots": screenshots,
            "observed_maxima": observed_maxima,
        }
    )
    return {
        "schema": BENCHMARK_SCHEMA,
        "status": (
            "calibrated" if pixel_evidence["status"] == "passed" else "failed"
        ),
        "client": client,
        "scope": (
            "same-client, same-binary repeated anchor noise calibration; "
            "the frozen cross-client thresholds are not derived or loosened here"
        ),
        "formal_timing_sample": False,
        "contracts": contracts,
        "client_identity": baseline_identity,
        "server_camera_log": {
            "sha256": baseline_camera.get("sha256"),
            "markers": baseline_camera.get("markers"),
        },
        "comparison_labels": {"vanilla": "baseline", "bbb": "repeat"},
        "observed_maxima": observed_maxima,
        "pixel_evidence": pixel_evidence,
        "visual_calibration_hash": evidence_hash,
    }


def validate_formal_evidence_linkage(
    *,
    contracts: Mapping[str, Any],
    camera_hash: str,
    workload: Mapping[str, Any],
    renderdoc: Mapping[str, Any],
) -> None:
    if workload.get("contracts") != dict(contracts):
        raise BenchmarkFailure(
            "visible workload evidence does not match the formal run contracts"
        )
    workload_camera = workload.get("server_camera_log")
    if (
        not isinstance(workload_camera, Mapping)
        or workload_camera.get("sha256") != camera_hash
    ):
        raise BenchmarkFailure(
            "visible workload evidence does not match the formal camera log"
        )

    screenshot_ticks: dict[tuple[str, str], int] = {}
    for pair in workload.get("screenshot_pairs", ()):
        if not isinstance(pair, Mapping):
            raise BenchmarkFailure("visible workload screenshot pair is invalid")
        segment = str(pair.get("capture_segment"))
        for client in ("vanilla", "bbb"):
            screenshot = pair.get(client)
            if not isinstance(screenshot, Mapping):
                raise BenchmarkFailure(
                    f"visible workload lacks {client}/{segment} screenshot"
                )
            try:
                screenshot_ticks[(client, segment)] = int(
                    screenshot["server_tick"]
                )
            except (KeyError, TypeError, ValueError) as error:
                raise BenchmarkFailure(
                    f"visible workload has invalid {client}/{segment} tick"
                ) from error

    renderdoc_ticks: dict[tuple[str, str], int] = {}
    for capture in renderdoc.get("captures", ()):
        if not isinstance(capture, Mapping):
            raise BenchmarkFailure("RenderDoc evidence capture is invalid")
        key = (str(capture.get("client")), str(capture.get("capture_segment")))
        if key in renderdoc_ticks:
            raise BenchmarkFailure(
                f"RenderDoc evidence duplicates {key[0]}/{key[1]}"
            )
        try:
            renderdoc_ticks[key] = int(capture["server_tick"])
        except (KeyError, TypeError, ValueError) as error:
            raise BenchmarkFailure(
                f"RenderDoc evidence has invalid {key[0]}/{key[1]} tick"
            ) from error
    if renderdoc_ticks != screenshot_ticks:
        raise BenchmarkFailure(
            "RenderDoc draw evidence does not match the reviewed screenshot anchors"
        )


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


def build_renderdoc_launch_argv(
    *,
    renderdoccmd: Path,
    capture_template: Path,
    working_dir: Path,
    client_argv: Sequence[str],
) -> tuple[str, ...]:
    if not renderdoccmd.is_file() or not os.access(renderdoccmd, os.X_OK):
        raise BenchmarkFailure(f"RenderDoc command is unavailable: {renderdoccmd}")
    if not client_argv:
        raise BenchmarkFailure("RenderDoc client command must not be empty")
    return (
        str(renderdoccmd),
        "capture",
        "--working-dir",
        str(working_dir),
        "--capture-file",
        str(capture_template),
        *client_argv,
    )


def parse_renderdoc_target_ident(text: str) -> int:
    matches = re.findall(r"^Launched as ID ([0-9]+)$", text, flags=re.MULTILINE)
    if len(matches) != 1:
        raise BenchmarkFailure(
            "RenderDoc launch output must contain exactly one target ID"
        )
    ident = int(matches[0])
    if not 1 <= ident <= 65535:
        raise BenchmarkFailure(f"invalid RenderDoc target ID: {ident}")
    return ident


def parse_renderdoc_control_output(text: str) -> dict[str, Any]:
    fields: dict[str, str] = {}
    for line in text.splitlines():
        key, separator, value = line.partition("=")
        if separator and re.fullmatch(r"[a-z_]+", key):
            fields[key] = value
    required = {
        "api",
        "byte_size",
        "capture_id",
        "frame_number",
        "path",
        "pid",
        "target",
    }
    missing = sorted(required - fields.keys())
    if missing:
        raise BenchmarkFailure(
            "RenderDoc target-control output is missing fields: "
            + ", ".join(missing)
        )
    try:
        numeric = {
            key: int(fields[key])
            for key in ("byte_size", "capture_id", "frame_number", "pid")
        }
    except ValueError as error:
        raise BenchmarkFailure(
            "RenderDoc target-control output contains an invalid integer"
        ) from error
    if numeric["byte_size"] <= 0 or numeric["pid"] <= 0:
        raise BenchmarkFailure(
            "RenderDoc capture size and target PID must be positive"
        )
    capture_path = Path(fields["path"])
    if not capture_path.is_absolute():
        raise BenchmarkFailure("RenderDoc capture path must be absolute")
    return {
        "api": fields["api"],
        **numeric,
        "path": str(capture_path),
        "target": fields["target"],
    }


def parse_renderdoc_xml(path: Path) -> dict[str, Any]:
    """Summarize RenderDoc's replay-independent XML event stream."""
    if not path.is_file():
        raise BenchmarkFailure(f"RenderDoc XML does not exist: {path}")
    driver: str | None = None
    renderer: str | None = None
    thumbnail: list[int] | None = None
    chunk_count = 0
    categories = {
        "draw": 0,
        "dispatch": 0,
        "clear": 0,
        "copy_or_blit": 0,
        "pass_boundary": 0,
        "present": 0,
    }
    draw_calls: dict[str, int] = {}
    try:
        elements = ElementTree.iterparse(path, events=("end",))
        for _event, element in elements:
            if element.tag == "driver" and driver is None:
                driver = (element.text or "").strip()
            elif element.tag == "thumbnail" and thumbnail is None:
                try:
                    thumbnail = [
                        int(element.attrib["width"]),
                        int(element.attrib["height"]),
                    ]
                except (KeyError, ValueError) as error:
                    raise BenchmarkFailure(
                        "RenderDoc XML thumbnail has invalid dimensions"
                    ) from error
            elif (
                element.tag == "string"
                and element.attrib.get("name") in ("renderer", "deviceName")
                and renderer is None
            ):
                renderer = (element.text or "").strip()
            elif element.tag == "chunk":
                chunk_count += 1
                name = element.attrib.get("name", "")
                is_draw = (
                    (
                        name.startswith(("glDraw", "glMultiDraw"))
                        and not name.startswith(("glDrawBuffer", "glDrawBuffers"))
                    )
                    or name.startswith("vkCmdDraw")
                )
                if is_draw:
                    categories["draw"] += 1
                    draw_calls[name] = draw_calls.get(name, 0) + 1
                if name.startswith(("glDispatchCompute", "vkCmdDispatch")):
                    categories["dispatch"] += 1
                if name.startswith(("glClear", "vkCmdClear")):
                    categories["clear"] += 1
                if name.startswith(
                    (
                        "glBlit",
                        "glCopy",
                        "vkCmdBlit",
                        "vkCmdCopy",
                        "vkCmdResolve",
                    )
                ):
                    categories["copy_or_blit"] += 1
                if name.startswith(
                    (
                        "vkCmdBeginRenderPass",
                        "vkCmdEndRenderPass",
                        "vkCmdBeginRendering",
                        "vkCmdEndRendering",
                    )
                ):
                    categories["pass_boundary"] += 1
                if name in (
                    "glXSwapBuffers",
                    "eglSwapBuffers",
                    "vkQueuePresentKHR",
                ):
                    categories["present"] += 1
            element.clear()
    except ElementTree.ParseError as error:
        raise BenchmarkFailure(f"invalid RenderDoc XML: {path}: {error}") from error
    if driver not in ("OpenGL", "Vulkan"):
        raise BenchmarkFailure(
            f"RenderDoc XML has unsupported or missing driver: {driver!r}"
        )
    if thumbnail is None or min(thumbnail) <= 0:
        raise BenchmarkFailure("RenderDoc XML lacks a positive thumbnail size")
    if categories["draw"] <= 0 or categories["present"] != 1:
        raise BenchmarkFailure(
            "RenderDoc XML must contain draw work and exactly one presentation"
        )
    return {
        "driver": driver,
        "renderer": renderer,
        "thumbnail": thumbnail,
        "chunk_count": chunk_count,
        "event_categories": categories,
        "draw_calls_by_api": dict(sorted(draw_calls.items())),
        "scope": (
            "replay-independent serialized API event counts; draw calls are "
            "not semantic workload equivalence, CPU duration, or GPU duration"
        ),
    }


def summarize_renderdoc_evidence(manifest: Mapping[str, Any]) -> dict[str, Any]:
    if manifest.get("schema") != BENCHMARK_SCHEMA:
        raise BenchmarkFailure("RenderDoc evidence manifest schema mismatch")
    captures = manifest.get("captures")
    if not isinstance(captures, list):
        raise BenchmarkFailure("RenderDoc evidence manifest lacks captures")
    capture_binary = manifest.get("bbb_binary")
    if not isinstance(capture_binary, Mapping) or not isinstance(
        capture_binary.get("sha256"), str
    ):
        raise BenchmarkFailure("RenderDoc evidence manifest lacks BBB binary identity")
    provenance_value = manifest.get("bbb_provenance")
    if not isinstance(provenance_value, Mapping):
        raise BenchmarkFailure("RenderDoc evidence manifest lacks BBB provenance")
    provenance = validate_bbb_binary_provenance(
        provenance_value,
        expected_capture_commit=str(manifest.get("bbb_commit")),
        expected_capture_sha256=str(capture_binary["sha256"]),
    )
    segments = (
        "static_orbit",
        "preloaded_flight",
        "chunk_boundary_crossing",
    )
    expected = {
        (client, segment)
        for client in ("vanilla", "bbb")
        for segment in segments
    }
    observed: dict[tuple[str, str], dict[str, Any]] = {}
    for capture in captures:
        if not isinstance(capture, Mapping):
            raise BenchmarkFailure("RenderDoc capture entry must be an object")
        client = capture.get("client")
        segment = capture.get("capture_segment")
        key = (client, segment)
        if key not in expected or key in observed:
            raise BenchmarkFailure(
                f"unexpected or duplicate RenderDoc capture: {client}/{segment}"
            )
        rdc_path = Path(str(capture.get("rdc")))
        xml_path = Path(str(capture.get("xml")))
        if not rdc_path.is_file() or rdc_path.stat().st_size <= 0:
            raise BenchmarkFailure(f"RenderDoc capture is unavailable: {rdc_path}")
        parsed = parse_renderdoc_xml(xml_path)
        expected_driver = "OpenGL" if client == "vanilla" else "Vulkan"
        if parsed["driver"] != expected_driver:
            raise BenchmarkFailure(
                f"{client} RenderDoc driver mismatch: {parsed['driver']}"
            )
        if parsed["thumbnail"] != [WINDOW_WIDTH, WINDOW_HEIGHT]:
            raise BenchmarkFailure(
                f"{client} RenderDoc frame is not {WINDOW_WIDTH}x{WINDOW_HEIGHT}"
            )
        try:
            server_tick = int(capture["server_tick"])
        except (KeyError, TypeError, ValueError) as error:
            raise BenchmarkFailure(
                f"{client}/{segment} lacks a valid server tick"
            ) from error
        observed[key] = {
            "client": client,
            "capture_segment": segment,
            "server_tick": server_tick,
            "rdc": {
                "path": str(rdc_path),
                "bytes": rdc_path.stat().st_size,
                "sha256": file_digest(rdc_path, "sha256"),
            },
            "xml": {
                "path": str(xml_path),
                "bytes": xml_path.stat().st_size,
                "sha256": file_digest(xml_path, "sha256"),
            },
            **parsed,
        }
    if set(observed) != expected:
        missing = sorted(expected - set(observed))
        raise BenchmarkFailure(f"RenderDoc evidence lacks captures: {missing}")
    ordered = [
        observed[(client, segment)]
        for segment in segments
        for client in ("vanilla", "bbb")
    ]
    evidence_hash = canonical_sha256(
        [
            {
                "client": item["client"],
                "capture_segment": item["capture_segment"],
                "server_tick": item["server_tick"],
                "rdc_sha256": item["rdc"]["sha256"],
                "xml_sha256": item["xml"]["sha256"],
                "driver": item["driver"],
                "thumbnail": item["thumbnail"],
                "event_categories": item["event_categories"],
                "draw_calls_by_api": item["draw_calls_by_api"],
            }
            for item in ordered
        ]
    )
    return {
        "schema": BENCHMARK_SCHEMA,
        "bbb_commit": provenance["evidence_commit"],
        "bbb_capture_commit": provenance["capture_commit"],
        "bbb_binary_sha256": provenance["binary_sha256"],
        "bbb_provenance": provenance,
        "status": "validated",
        "formal_timing_sample": False,
        "capture_overhead_excluded_from_metrics": True,
        "renderdoc_evidence_hash": evidence_hash,
        "captures": ordered,
        "scope": (
            "common RenderDoc 1.45 raw frame captures and replay-independent "
            "XML API event streams prove each anchor submitted visible draw "
            "work; API call counts are not compared as equivalent semantics "
            "and are not performance measurements"
        ),
    }


def validate_gpu_duration_summary(
    summary: Mapping[str, Any],
    *,
    client_pid: int,
    minimum_association_ratio: float,
) -> dict[str, Any]:
    if summary.get("schema") != BENCHMARK_SCHEMA:
        raise BenchmarkFailure("GPU duration summary schema mismatch")
    if summary.get("metric") != "per-client GPU execution duration":
        raise BenchmarkFailure("GPU duration summary metric mismatch")
    if summary.get("chain") != list(GPU_DURATION_CHAIN_EVENTS):
        raise BenchmarkFailure("GPU duration summary chain mismatch")
    if summary.get("graphics_only") is not True:
        raise BenchmarkFailure("GPU duration summary is not graphics-only")
    try:
        association_ratio = float(summary["association_ratio"])
    except (KeyError, TypeError, ValueError) as error:
        raise BenchmarkFailure(
            "GPU duration summary lacks an association ratio"
        ) from error
    if (
        not math.isfinite(association_ratio)
        or not 0.0 <= minimum_association_ratio <= 1.0
        or association_ratio < minimum_association_ratio
    ):
        raise BenchmarkFailure(
            "GPU duration association ratio "
            f"{association_ratio:.6f} is below "
            f"{minimum_association_ratio:.6f}"
        )
    clients = summary.get("clients")
    if not isinstance(clients, list) or len(clients) != 1:
        raise BenchmarkFailure(
            "GPU duration summary must contain exactly one client"
        )
    client = clients[0]
    if not isinstance(client, Mapping) or client.get("pid") != client_pid:
        raise BenchmarkFailure("GPU duration client PID mismatch")
    execution = client.get("gpu_execution")
    queue_wait = client.get("queue_wait")
    if not isinstance(execution, Mapping) or not isinstance(queue_wait, Mapping):
        raise BenchmarkFailure("GPU duration summary lacks duration statistics")
    try:
        execution_count = int(execution["count"])
        queue_count = int(queue_wait["count"])
        job_count = int(summary["job_count"])
        submitted_jobs = int(summary["submitted_jobs"])
        associated_jobs = int(summary["associated_jobs"])
    except (KeyError, TypeError, ValueError) as error:
        raise BenchmarkFailure("GPU duration summary has invalid counts") from error
    if (
        execution_count <= 0
        or execution_count != queue_count
        or execution_count != job_count
        or job_count > associated_jobs
        or not 0 <= associated_jobs <= submitted_jobs
        or submitted_jobs < job_count
        or not math.isclose(
            association_ratio,
            associated_jobs / submitted_jobs,
            rel_tol=1e-9,
            abs_tol=1e-12,
        )
    ):
        raise BenchmarkFailure("GPU duration summary job counts mismatch")
    return {
        "association_ratio": association_ratio,
        "submitted_jobs": submitted_jobs,
        "associated_jobs": associated_jobs,
        "job_count": job_count,
        "comm": client.get("comm"),
        "rings": client.get("rings"),
        "gpu_execution": dict(execution),
        "queue_wait": dict(queue_wait),
    }


def summarize_formal_comparison(manifest: Mapping[str, Any]) -> dict[str, Any]:
    if manifest.get("schema") != BENCHMARK_SCHEMA:
        raise BenchmarkFailure("formal comparison manifest schema mismatch")
    run_paths = manifest.get("runs")
    if not isinstance(run_paths, list) or len(run_paths) < 6:
        raise BenchmarkFailure("formal comparison requires at least six runs")
    runs = [
        load_json_object(Path(str(path)), "formal capture run")
        for path in run_paths
    ]
    clients = [str(run.get("client")) for run in runs]
    if len(runs) % 4 != 0 or any(
        clients[index : index + 4] != ["vanilla", "bbb", "bbb", "vanilla"]
        for index in range(0, len(runs), 4)
    ):
        raise BenchmarkFailure("formal capture order is not repeated ABBA")
    if min(clients.count("vanilla"), clients.count("bbb")) < 3:
        raise BenchmarkFailure("formal comparison lacks three runs per client")

    provenance_value = manifest.get("bbb_provenance")
    if not isinstance(provenance_value, Mapping):
        raise BenchmarkFailure("formal comparison manifest lacks BBB provenance")
    expected_binary = manifest.get("bbb_binary_sha256")
    if not isinstance(expected_binary, str):
        raise BenchmarkFailure("formal comparison manifest lacks BBB binary identity")
    provenance = validate_bbb_binary_provenance(
        provenance_value,
        expected_capture_commit=str(manifest.get("bbb_commit")),
        expected_capture_sha256=expected_binary,
    )

    workload_path = Path(str(manifest.get("workload_evidence")))
    renderdoc_path = Path(str(manifest.get("renderdoc_evidence")))
    workload = load_json_object(workload_path, "workload evidence")
    renderdoc = load_json_object(renderdoc_path, "RenderDoc evidence")
    if workload.get("status") != "matched":
        raise BenchmarkFailure("visible workload evidence is not matched")
    if renderdoc.get("status") != "validated":
        raise BenchmarkFailure("common draw evidence is not validated")
    for label, evidence in (("workload", workload), ("RenderDoc", renderdoc)):
        if evidence.get("bbb_provenance") != provenance:
            raise BenchmarkFailure(f"{label} BBB binary provenance mismatch")

    contracts: dict[str, Any] = {}
    contract_keys = (
        "replay_hash",
        "world_hash",
        "resource_hash",
        "settings_hash",
        "scene_contract_hash",
    )
    for key in contract_keys:
        values = {run.get(key) for run in runs}
        if len(values) != 1 or None in values:
            raise BenchmarkFailure(f"formal run {key} mismatch")
        contracts[key] = next(iter(values))
    camera_hashes = {
        run.get("server_camera_log", {}).get("sha256") for run in runs
    }
    if len(camera_hashes) != 1 or None in camera_hashes:
        raise BenchmarkFailure("formal run camera logs differ")
    camera_hash = str(next(iter(camera_hashes)))
    validate_formal_evidence_linkage(
        contracts=contracts,
        camera_hash=camera_hash,
        workload=workload,
        renderdoc=renderdoc,
    )

    metric_names = (
        "frame_ms",
        "gpu_load",
        "gpu_clock_mhz",
        "gpu_temp_c",
        "cpu_temp_c",
    )
    aggregate_captures: dict[
        str, dict[str, dict[str, list[float]]]
    ] = {
        client: {
            segment: {name: [] for name in metric_names}
            for segment in ("overall", *FORMAL_CAPTURE_SEGMENTS)
        }
        for client in ("vanilla", "bbb")
    }
    perf_totals = {
        client: {event: 0.0 for event in REQUIRED_PERF_EVENTS}
        for client in ("vanilla", "bbb")
    }
    presented = {"vanilla": 0, "bbb": 0}
    run_evidence: list[dict[str, Any]] = []
    replay = load_json_object(
        Path(str(manifest.get("replay"))), "formal replay"
    )
    for index, (path_value, run) in enumerate(zip(run_paths, runs), start=1):
        client = str(run["client"])
        if run.get("status") != "passed":
            raise BenchmarkFailure(f"formal run {index} did not pass")
        if run.get("position_corrections") != 0:
            raise BenchmarkFailure(f"formal run {index} has a position correction")
        if run.get("throttling") is not False:
            raise BenchmarkFailure(
                f"formal run {index} has throttling or unknown thermal state"
            )
        expected_api = "opengl" if client == "vanilla" else "vulkan"
        if run.get("capture_api") != expected_api:
            raise BenchmarkFailure(f"formal run {index} capture API mismatch")
        raw_logs = run.get("mangohud_raw_logs")
        if not isinstance(raw_logs, list) or len(raw_logs) != 1:
            raise BenchmarkFailure(
                f"formal run {index} lacks one MangoHud raw log"
            )
        raw_path = Path(str(raw_logs[0]))
        capture = parse_mangohud_csv(raw_path)
        gpu = str(capture.get("system", {}).get("gpu", ""))
        if "AMD Radeon 610M" not in gpu or "llvmpipe" in gpu.lower():
            raise BenchmarkFailure(
                f"formal run {index} did not use the AMD hardware GPU"
            )
        marker_timeline = run.get("marker_timeline")
        if not isinstance(marker_timeline, list):
            raise BenchmarkFailure(
                f"formal run {index} lacks an observed marker timeline"
            )
        split = split_mangohud_capture_by_replay(
            capture, replay, marker_timeline
        )
        append_formal_segment_samples(
            aggregate_captures[client], split, metric_names
        )
        presented[client] += len(capture["frame_ms"])

        perf = run.get("perf_stat", {}).get("events")
        if not isinstance(perf, Mapping):
            raise BenchmarkFailure(f"formal run {index} lacks perf stat")
        for event in REQUIRED_PERF_EVENTS:
            try:
                perf_totals[client][event] += float(perf[event])
            except (KeyError, TypeError, ValueError) as error:
                raise BenchmarkFailure(
                    f"formal run {index} lacks perf event {event}"
                ) from error

        if client == "vanilla":
            options = run.get("loaded_options", {}).get("values", {})
            if options.get("inactivityFpsLimit") != "minimized":
                raise BenchmarkFailure(
                    f"formal run {index} did not disable the AFK limiter"
                )
        else:
            launch = load_json_object(
                Path(str(run.get("launch_manifest"))), "bbb launch manifest"
            )
            if launch.get("binary", {}).get("sha256") != expected_binary:
                raise BenchmarkFailure(f"formal run {index} bbb binary mismatch")
        raw_summary = asdict(summarize_frames(capture))
        run_evidence.append(
            {
                "sequence": index,
                "client": client,
                "smoke": str(path_value),
                "smoke_sha256": file_digest(Path(str(path_value)), "sha256"),
                "raw": str(raw_path),
                "raw_sha256": file_digest(raw_path, "sha256"),
                "gpu": gpu,
                "frames": raw_summary,
                "perf": run["perf_stat"],
                "server_camera_log": run["server_camera_log"],
            }
        )

    overhead_manifest = manifest.get("mangohud_overhead")
    if not isinstance(overhead_manifest, Mapping):
        raise BenchmarkFailure("formal manifest lacks MangoHud overhead evidence")
    threshold = float(overhead_manifest.get("threshold_percent", 1.0))
    overhead: dict[str, Any] = {}
    for client in ("vanilla", "bbb"):
        entry = overhead_manifest.get(client)
        if not isinstance(entry, Mapping):
            raise BenchmarkFailure(f"missing {client} MangoHud overhead evidence")
        if entry.get("baseline_smoke"):
            baseline_smoke = load_json_object(
                Path(str(entry["baseline_smoke"])), "overhead baseline"
            )
            baseline = baseline_smoke.get("weston_timeline")
        else:
            baseline = parse_weston_timeline(
                Path(str(entry.get("baseline_timeline"))),
                surface_title=str(entry.get("surface_title")),
            )
        instrumented_smoke = load_json_object(
            Path(str(entry.get("instrumented_smoke"))),
            "overhead instrumented run",
        )
        instrumented = instrumented_smoke.get("weston_timeline")
        if not isinstance(baseline, Mapping) or not isinstance(
            instrumented, Mapping
        ):
            raise BenchmarkFailure(f"{client} overhead timeline is incomplete")
        baseline_ms = float(baseline["commit_interval"]["p50_ms"])
        instrumented_ms = float(
            instrumented["commit_interval"]["p50_ms"]
        )
        impact = (instrumented_ms - baseline_ms) * 100.0 / baseline_ms
        if impact > threshold:
            raise BenchmarkFailure(
                f"{client} MangoHud median overhead {impact:.3f}% exceeds "
                f"{threshold:.3f}%"
            )
        overhead[client] = {
            "baseline_p50_ms": baseline_ms,
            "instrumented_p50_ms": instrumented_ms,
            "impact_percent": impact,
            "threshold_percent": threshold,
            "scope": baseline["scope"],
        }

    gpu_duration_manifest = manifest.get("gpu_duration_evidence")
    if not isinstance(gpu_duration_manifest, Mapping):
        raise BenchmarkFailure("formal manifest lacks GPU duration evidence")
    gpu_overhead_threshold = float(
        gpu_duration_manifest.get("threshold_percent", 1.0)
    )
    minimum_association_ratio = float(
        gpu_duration_manifest.get("minimum_association_ratio", 0.99)
    )
    gpu_duration_clients: dict[str, Any] = {}
    for client in ("vanilla", "bbb"):
        entry = gpu_duration_manifest.get(client)
        if not isinstance(entry, Mapping):
            raise BenchmarkFailure(f"missing {client} GPU duration evidence")
        baseline_path = Path(str(entry.get("baseline_smoke")))
        instrumented_path = Path(str(entry.get("instrumented_smoke")))
        summary_path = Path(str(entry.get("summary")))
        baseline_smoke = load_json_object(
            baseline_path, f"{client} GPU overhead baseline"
        )
        instrumented_smoke = load_json_object(
            instrumented_path, f"{client} GPU duration run"
        )
        for label, smoke in (
            ("baseline", baseline_smoke),
            ("instrumented", instrumented_smoke),
        ):
            if smoke.get("status") != "passed" or smoke.get("client") != client:
                raise BenchmarkFailure(
                    f"{client} GPU {label} smoke identity mismatch"
                )
            if any(smoke.get(key) != value for key, value in contracts.items()):
                raise BenchmarkFailure(
                    f"{client} GPU {label} smoke contract mismatch"
                )
            if smoke.get("server_camera_log", {}).get("sha256") != camera_hash:
                raise BenchmarkFailure(
                    f"{client} GPU {label} camera log mismatch"
                )
            if client == "bbb":
                launch = load_json_object(
                    Path(str(smoke.get("launch_manifest"))),
                    f"{client} GPU {label} launch manifest",
                )
                if launch.get("binary", {}).get("sha256") != expected_binary:
                    raise BenchmarkFailure(
                        f"{client} GPU {label} bbb binary mismatch"
                    )
        baseline_timeline = baseline_smoke.get("weston_timeline")
        instrumented_timeline = instrumented_smoke.get("weston_timeline")
        if not isinstance(baseline_timeline, Mapping) or not isinstance(
            instrumented_timeline, Mapping
        ):
            raise BenchmarkFailure(f"{client} GPU overhead timeline is incomplete")
        baseline_ms = float(
            baseline_timeline["commit_interval"]["p50_ms"]
        )
        instrumented_ms = float(
            instrumented_timeline["commit_interval"]["p50_ms"]
        )
        impact = (instrumented_ms - baseline_ms) * 100.0 / baseline_ms
        if impact > gpu_overhead_threshold:
            raise BenchmarkFailure(
                f"{client} GPU trace median overhead {impact:.3f}% exceeds "
                f"{gpu_overhead_threshold:.3f}%"
            )
        duration_summary = load_json_object(
            summary_path, f"{client} GPU duration summary"
        )
        validated_duration = validate_gpu_duration_summary(
            duration_summary,
            client_pid=int(instrumented_smoke["client_pid"]),
            minimum_association_ratio=minimum_association_ratio,
        )
        gpu_duration_clients[client] = {
            **validated_duration,
            "summary": str(summary_path),
            "summary_sha256": file_digest(summary_path, "sha256"),
            "overhead": {
                "baseline_p50_ms": baseline_ms,
                "instrumented_p50_ms": instrumented_ms,
                "impact_percent": impact,
                "threshold_percent": gpu_overhead_threshold,
                "scope": baseline_timeline["scope"],
            },
        }

    performance: dict[str, Any] = {}
    for client in ("vanilla", "bbb"):
        performance[client] = {
            "frames": {
                segment: asdict(
                    summarize_frames(capture)
                )
                for segment, capture in aggregate_captures[client].items()
            },
            "perf": {
                "events": perf_totals[client],
                "presented_frames": presented[client],
                "per_presented_frame": {
                    event: value / presented[client]
                    for event, value in perf_totals[client].items()
                },
                "scope": (
                    "process totals and per-MangoHud-frame means; "
                    "no CPU-frame percentiles"
                ),
            },
        }
    return {
        "schema": BENCHMARK_SCHEMA,
        "bbb_commit": provenance["evidence_commit"],
        "bbb_capture_commit": provenance["capture_commit"],
        "bbb_binary_sha256": provenance["binary_sha256"],
        "bbb_provenance": provenance,
        "formal_comparison_complete": True,
        "metrics_publishable": True,
        "contracts": contracts,
        "server_camera_log_sha256": camera_hash,
        "mangohud_overhead": overhead,
        "runs": run_evidence,
        "performance": performance,
        "metric_scope": (
            "MangoHud present-call frame intervals p50/p95/p99, 1% low, "
            "hitches and GPU telemetry; perf process totals are normalized "
            "per captured frame and are not CPU-frame percentiles"
        ),
        "gpu_duration": {
            "claim_allowed": True,
            "minimum_association_ratio": minimum_association_ratio,
            "clients": gpu_duration_clients,
            "scope": (
                "per-client graphics-ring GPU execution duration joined from "
                "amdgpu submission through scheduler dispatch to finished "
                "dma_fence signal; queue wait is reported separately and "
                "compositor submissions are excluded by client PID"
            ),
        },
        "artifacts": {
            "workload_evidence": str(workload_path),
            "workload_evidence_sha256": file_digest(
                workload_path, "sha256"
            ),
            "renderdoc_evidence": str(renderdoc_path),
            "renderdoc_evidence_sha256": file_digest(
                renderdoc_path, "sha256"
            ),
        },
    }


def _capture_renderdoc_anchor(
    *,
    control_path: Path,
    capture_dir: Path,
    target_ident: int,
    client: str,
    segment: str,
    server_tick: int,
) -> dict[str, Any]:
    if not control_path.is_file() or not os.access(control_path, os.X_OK):
        raise BenchmarkFailure(
            f"RenderDoc target-control helper is unavailable: {control_path}"
        )
    before = set(capture_dir.glob("*.rdc"))
    capture = subprocess.run(
        (str(control_path), str(target_ident), "0"),
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=45.0,
        check=False,
    )
    if capture.returncode != 0:
        raise BenchmarkFailure(
            "RenderDoc target-control capture failed: "
            + (capture.stdout.strip() or f"exit {capture.returncode}")
        )
    parsed = parse_renderdoc_control_output(capture.stdout)
    created = set(capture_dir.glob("*.rdc")) - before
    if len(created) != 1:
        raise BenchmarkFailure(
            "RenderDoc target-control must create exactly one new capture; "
            f"observed {len(created)}"
        )
    path = created.pop()
    actual_size = path.stat().st_size
    frame_match = re.search(r"_frame([0-9]+)\.rdc$", path.name)
    if frame_match is None:
        raise BenchmarkFailure(
            f"RenderDoc capture filename has no frame number: {path}"
        )
    return {
        **parsed,
        "byte_size": actual_size,
        "frame_number": int(frame_match.group(1)),
        "path": str(path),
        "client": client,
        "capture_segment": segment,
        "server_tick": server_tick,
        "sha256": file_digest(path, "sha256"),
    }


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
    vanilla_root: Path,
    java_path: Path,
    world_manifest_path: Path,
    artifact_dir: Path,
    server_port: int,
    accept_eula: bool,
    timeout: float,
    wayland_display: str,
    x_display: str | None,
    mangohud_root: Path | None,
    perf_path: Path | None,
    weston_debug_path: Path | None,
    weston_path: Path | None,
    workload_debug_profile: bool,
    weston_renderer: str,
    renderdoccmd: Path | None,
    renderdoc_control: Path | None,
    renderdoc_segments: frozenset[str] | None = None,
) -> dict[str, Any]:
    if weston_renderer not in ("gl", "pixman"):
        raise BenchmarkFailure(f"unsupported Weston renderer: {weston_renderer}")
    if renderdoccmd is not None and mangohud_root is not None:
        raise BenchmarkFailure(
            "RenderDoc validation and MangoHud timing must use separate runs"
        )
    if renderdoc_segments is not None and renderdoccmd is None:
        raise BenchmarkFailure("RenderDoc segment selection requires RenderDoc")
    if perf_path is not None and mangohud_root is None:
        raise BenchmarkFailure(
            "perf normalization requires a simultaneous MangoHud frame capture"
        )
    if weston_debug_path is not None and x_display is None:
        raise BenchmarkFailure(
            "Weston timeline overhead capture requires the Xwayland path"
        )
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

    audit = audit_distribution(vanilla_root, java_path=java_path)
    server = validate_server_jar(vanilla_root)
    natives_dir = artifact_dir / "natives"
    natives = safe_extract_natives(
        (Path(entry.path) for entry in audit.native_classifiers), natives_dir
    )
    game_dir = artifact_dir / "game"
    options_path = write_vanilla_options(game_dir)
    debug_profile_path = (
        write_vanilla_workload_debug_profile(game_dir)
        if workload_debug_profile
        else None
    )
    plan = build_vanilla_launch_plan(
        audit,
        java_path=java_path,
        game_dir=game_dir,
        assets_dir=vanilla_root / "downloads" / MINECRAFT_VERSION / "assets",
        natives_dir=natives_dir,
        server_port=server_port,
        prefer_wayland=x_display is None,
    )
    server_log_path = artifact_dir / "server.log"
    client_log_path = artifact_dir / "vanilla-client.log"
    evidence_path = artifact_dir / "smoke.json"
    launch_path = artifact_dir / "vanilla-launch.json"
    host = host_facts(java_path)
    host["window_system"] = "x11" if x_display is not None else "wayland"
    host["display"] = x_display
    host["wayland_display"] = None if x_display is not None else wayland_display
    client_argv = plan.argv
    if renderdoccmd is not None:
        if renderdoc_control is None:
            raise BenchmarkFailure(
                "RenderDoc target-control helper is required for capture"
            )
        renderdoc_dir = artifact_dir / "renderdoc"
        renderdoc_dir.mkdir()
        client_argv = build_renderdoc_launch_argv(
            renderdoccmd=renderdoccmd,
            capture_template=renderdoc_dir / "vanilla",
            working_dir=game_dir,
            client_argv=plan.argv,
        )
    launch_path.write_bytes(
        canonical_json_bytes(
            {
                "schema": BENCHMARK_SCHEMA,
                "audit": asdict(audit),
                "natives": [asdict(entry) for entry in natives],
                "options_hash": file_digest(options_path, "sha256"),
                "workload_debug_profile": (
                    {
                        "path": str(debug_profile_path),
                        "sha256": file_digest(debug_profile_path, "sha256"),
                    }
                    if debug_profile_path is not None
                    else None
                ),
                "launch": {**asdict(plan), "argv": sanitized_argv(client_argv)},
                "renderdoc": (
                    {
                        "command": str(renderdoccmd),
                        "command_sha256": file_digest(renderdoccmd, "sha256"),
                        "control": str(renderdoc_control),
                        "control_sha256": file_digest(renderdoc_control, "sha256"),
                    }
                    if renderdoccmd is not None and renderdoc_control is not None
                    else None
                ),
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
        "workload_debug_profile": workload_debug_profile,
        "formal_capture_valid": False,
        "scope": "launcher/replay/common-capture semantic smoke, not a formal performance run",
        "server": asdict(server),
        "server_port": server_port,
        "launch_manifest": str(launch_path),
        "server_log": str(server_log_path),
        "client_log": str(client_log_path),
        "screenshot": None,
        "screenshots": [],
        "renderdoc_captures": [],
        "renderdoc_segments": (
            sorted(renderdoc_segments) if renderdoc_segments is not None else None
        ),
        "wayland_display": wayland_display,
        "x_display": x_display,
        "window_system": "x11" if x_display is not None else "wayland",
        "resolution": [WINDOW_WIDTH, WINDOW_HEIGHT],
    }
    server_group: ProcessGroup | None = None
    client_group: ProcessGroup | None = None
    perf_session: PerfStatSession | None = None
    timeline_session: WestonTimelineSession | None = None
    compositor_group: ProcessGroup | None = None
    compositor_output: Any = None
    try:
        if re.fullmatch(r"[A-Za-z0-9_.-]+", wayland_display) is None:
            raise BenchmarkFailure(
                f"unsafe Wayland display name: {wayland_display!r}"
            )
        if x_display is not None and re.fullmatch(
            r":[0-9]+(?:\\.[0-9]+)?", x_display
        ) is None:
            raise BenchmarkFailure(f"unsafe X display name: {x_display!r}")
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
                f"--renderer={weston_renderer}",
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
            if x_display is None:
                environment["WAYLAND_DISPLAY"] = wayland_display
            else:
                environment.pop("WAYLAND_DISPLAY", None)
                environment["DISPLAY"] = x_display
            if mangohud_root is not None:
                mango_environment, mango_audit = mangohud_environment(
                    package_root=mangohud_root,
                    output_dir=artifact_dir / "mangohud",
                    api="opengl",
                )
                environment.update(mango_environment)
                result["mangohud"] = mango_audit
            client_group = ProcessGroup(
                client_argv,
                cwd=game_dir,
                stdin=subprocess.DEVNULL,
                stdout=client_log,
                stderr=subprocess.STDOUT,
                env=environment,
            )
            client_process = client_group.start()
            result["client_pid"] = client_process.pid
            client_watch_process = (
                None if renderdoccmd is not None else client_process
            )
            _wait_for_log(
                server_log_path,
                f"{BENCHMARK_USERNAME} joined the game",
                process=client_watch_process,
                timeout=timeout,
            )
            renderdoc_ident: int | None = None
            if renderdoccmd is not None:
                _wait_for_log(
                    client_log_path,
                    "Launched as ID ",
                    process=None,
                    timeout=timeout,
                )
                renderdoc_ident = parse_renderdoc_target_ident(
                    client_log_path.read_text(
                        encoding="utf-8", errors="replace"
                    )
                )
                result["renderdoc_target_ident"] = renderdoc_ident
            result["warmup_seconds"] = FIXED_WARMUP_SECONDS
            _warmup_and_start_replay(server_process, replay)
            capture_markers = replay_capture_markers(replay)
            first_capture_start = capture_markers[0]
            _wait_for_log(
                server_log_path,
                benchmark_marker_line(digest, first_capture_start),
                process=client_watch_process,
                timeout=timeout,
            )
            if weston_debug_path is not None:
                timeline_session = WestonTimelineSession(
                    weston_debug_path=weston_debug_path,
                    wayland_display=wayland_display,
                    output_path=artifact_dir / "weston-timeline.jsonl",
                    surface_title=VANILLA_WINDOW_TITLE,
                )
                result["weston_timeline_start"] = timeline_session.start()
            if mangohud_root is not None:
                if perf_path is not None:
                    perf_session = PerfStatSession(
                        perf_path=perf_path,
                        target_pid=client_process.pid,
                        output_path=artifact_dir / "perf-stat.csv",
                    )
                    result["perf_capture_start"] = perf_session.start()
                result["mangohud_capture_start"] = mangohud_set_logging(
                    process_pid=client_process.pid, enabled=True
                )
            capture_origin = time.monotonic()
            result["marker_timeline"] = [
                marker_timeline_observation(
                    first_capture_start, elapsed_seconds=0.0
                )
            ]
            for marker_frame in capture_markers[1:]:
                _wait_for_log(
                    server_log_path,
                    benchmark_marker_line(digest, marker_frame),
                    process=client_watch_process,
                    timeout=timeout,
                )
                result["marker_timeline"].append(
                    marker_timeline_observation(
                        marker_frame,
                        elapsed_seconds=time.monotonic() - capture_origin,
                    )
                )
                if marker_frame["marker"] != "anchor":
                    continue
                if (
                    renderdoc_ident is not None
                    and renderdoc_control is not None
                    and (
                        renderdoc_segments is None
                        or marker_frame["capture_segment"] in renderdoc_segments
                    )
                ):
                    result["renderdoc_captures"].append(
                        _capture_renderdoc_anchor(
                            control_path=renderdoc_control,
                            capture_dir=artifact_dir / "renderdoc",
                            target_ident=renderdoc_ident,
                            client="vanilla",
                            segment=marker_frame["capture_segment"],
                            server_tick=marker_frame["server_tick"],
                        )
                    )
                time.sleep(0.5)
                if weston_path is not None:
                    result["screenshots"].append(
                        _take_weston_screenshot(
                            artifact_dir=artifact_dir,
                            environment=environment,
                            client="vanilla",
                            segment=marker_frame["capture_segment"],
                            tick=marker_frame["server_tick"],
                        )
                    )
            if mangohud_root is not None:
                result["mangohud_capture_stop"] = mangohud_set_logging(
                    process_pid=client_process.pid,
                    enabled=False,
                    completion_dir=artifact_dir / "mangohud",
                )
                if perf_session is not None:
                    result["perf_capture_stop"] = perf_session.stop()
            if timeline_session is not None:
                result["weston_timeline"] = timeline_session.stop()
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
        if perf_session is not None and (
            perf_session.process is not None
            and perf_session.process.poll() is None
        ):
            try:
                result["perf_capture_stop"] = perf_session.stop()
            except BenchmarkFailure as error:
                result.setdefault("invalid_reasons", []).append(str(error))
        if timeline_session is not None and (
            timeline_session.process is not None
            and timeline_session.process.poll() is None
        ):
            try:
                result["weston_timeline"] = timeline_session.stop()
            except BenchmarkFailure as error:
                result.setdefault("invalid_reasons", []).append(str(error))
        if client_group is not None:
            if mangohud_root is not None:
                result["client_cleanup"] = {
                    "signal": "SIGKILL",
                    "reason": (
                        "MangoHud 0.8.4 OpenGL reproducibly crashes during "
                        "ordered application teardown; "
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
                    marker_timeline = result.get("marker_timeline")
                    if not isinstance(marker_timeline, list):
                        raise BenchmarkFailure(
                            "official capture lacks an observed marker timeline"
                        )
                    mango_summary = summarize_mangohud_logs(
                        raw_logs, replay, marker_timeline
                    )
                    result["mangohud_frame_summaries"] = mango_summary["overall"]
                    result["mangohud_segment_summaries"] = mango_summary["segments"]
                    result["presented_frames"] = mango_summary["presented_frames"]
                    result["capture_api"] = "opengl"
                    thermal = mango_summary["overall"][0]
                    peaks = (
                        thermal["cpu_temp_peak_c"],
                        thermal["gpu_temp_peak_c"],
                    )
                    result["throttling"] = (
                        None
                        if any(value is None for value in peaks)
                        else any(value >= THERMAL_REJECT_C for value in peaks)
                    )
                    result["thermal_reject_c"] = THERMAL_REJECT_C
                    perf_output = artifact_dir / "perf-stat.csv"
                    if perf_path is not None:
                        result["perf_stat"] = parse_perf_stat(
                            perf_output,
                            presented_frames=result["presented_frames"],
                        )
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


def bbb_terrain_residency_evidence(renderer: Mapping[str, Any]) -> dict[str, Any]:
    fields = {
        name: int(renderer.get(name, 0))
        for name in (
            "terrain_selected_chunks",
            "terrain_resident_chunks",
            "terrain_selected_sections",
            "terrain_resident_sections",
            "terrain_pending_jobs",
            "terrain_mesh_queue_depth",
            "uploaded_sections",
        )
    }
    ready = renderer.get("terrain_cache_ready") is True
    return {**fields, "terrain_cache_ready": ready}


def require_bbb_terrain_residency(
    renderer: Mapping[str, Any], *, capture_segment: str
) -> dict[str, Any]:
    evidence = bbb_terrain_residency_evidence(renderer)
    valid = (
        evidence["terrain_cache_ready"]
        and evidence["terrain_selected_chunks"] > 0
        and evidence["terrain_selected_chunks"]
        == evidence["terrain_resident_chunks"]
        and evidence["terrain_selected_sections"] > 0
        and evidence["terrain_selected_sections"]
        == evidence["terrain_resident_sections"]
        == evidence["uploaded_sections"]
        and evidence["terrain_pending_jobs"] == 0
        and evidence["terrain_mesh_queue_depth"] == 0
    )
    if not valid:
        evidence.update(
            {
                name: int(renderer.get(name, 0))
                for name in (
                    "terrain_mesh_jobs_queued",
                    "terrain_mesh_jobs_completed",
                    "terrain_mesh_jobs_stale",
                    "terrain_mesh_sections_queued",
                    "terrain_mesh_sections_completed",
                    "terrain_mesh_sections_stale",
                    "terrain_mesh_queue_full",
                    "terrain_mesh_recent_build_nanos",
                    "terrain_mesh_peak_build_nanos",
                    "terrain_mesh_recent_build_sections",
                    "terrain_mesh_peak_build_sections",
                    "terrain_changed_owners",
                    "terrain_uploaded_owners",
                )
            }
        )
        raise BenchmarkFailure(
            f"bbb {capture_segment} terrain is not fully resident: "
            f"{json.dumps(evidence, sort_keys=True)}"
        )
    return evidence


def bbb_renderdoc_residency_evidence(
    renderer: Mapping[str, Any],
    *,
    capture_segment: str,
    renderdoc_segments: frozenset[str] | None,
) -> dict[str, Any]:
    if renderdoc_segments is None or capture_segment in renderdoc_segments:
        return require_bbb_terrain_residency(
            renderer, capture_segment=capture_segment
        )
    return {
        **bbb_terrain_residency_evidence(renderer),
        "validation_scope": "observed-only; another segment owns this RenderDoc run",
    }


def run_bbb_smoke(
    *,
    replay: Mapping[str, Any],
    vanilla_root: Path,
    java_path: Path,
    bbb_binary: Path,
    world_manifest_path: Path,
    artifact_dir: Path,
    server_port: int,
    control_port: int,
    accept_eula: bool,
    timeout: float,
    wayland_display: str,
    x_display: str | None,
    mangohud_root: Path | None,
    perf_path: Path | None,
    weston_debug_path: Path | None,
    weston_path: Path | None,
    weston_renderer: str,
    renderdoccmd: Path | None,
    renderdoc_control: Path | None,
    extra_library_path: Path | None,
    renderdoc_segments: frozenset[str] | None = None,
) -> dict[str, Any]:
    if weston_renderer not in ("gl", "pixman"):
        raise BenchmarkFailure(f"unsupported Weston renderer: {weston_renderer}")
    if renderdoccmd is not None and mangohud_root is not None:
        raise BenchmarkFailure(
            "RenderDoc validation and MangoHud timing must use separate runs"
        )
    if renderdoc_segments is not None and renderdoccmd is None:
        raise BenchmarkFailure("RenderDoc segment selection requires RenderDoc")
    if perf_path is not None and mangohud_root is None:
        raise BenchmarkFailure(
            "perf normalization requires a simultaneous MangoHud frame capture"
        )
    if weston_debug_path is not None and x_display is None:
        raise BenchmarkFailure(
            "Weston timeline overhead capture requires the Xwayland path"
        )
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
    audit = audit_distribution(vanilla_root, java_path=java_path)
    server = validate_server_jar(vanilla_root)
    client_argv = build_bbb_launch_argv(
        binary=bbb_binary,
        vanilla_root=vanilla_root,
        server_port=server_port,
        control_port=control_port,
    )
    runtime_dir = Path(os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000"))
    if re.fullmatch(r"[A-Za-z0-9_.-]+", wayland_display) is None:
        raise BenchmarkFailure(f"unsafe Wayland display name: {wayland_display!r}")
    if x_display is not None and re.fullmatch(
        r":[0-9]+(?:\\.[0-9]+)?", x_display
    ) is None:
        raise BenchmarkFailure(f"unsafe X display name: {x_display!r}")
    if weston_path is not None and (
        not weston_path.is_file() or not os.access(weston_path, os.X_OK)
    ):
        raise BenchmarkFailure(f"Weston executable is unavailable: {weston_path}")
    socket_path = runtime_dir / wayland_display
    if weston_path is not None and socket_path.exists():
        raise BenchmarkFailure(f"refusing to replace existing Wayland socket: {socket_path}")
    if x_display is None and weston_path is None:
        raise BenchmarkFailure(
            "bbb Wayland smoke requires a managed Weston compositor"
        )
    if extra_library_path is not None and not extra_library_path.is_dir():
        raise BenchmarkFailure(
            f"extra library directory is unavailable: {extra_library_path}"
        )
    launch_client_argv = client_argv
    if renderdoccmd is not None:
        if renderdoc_control is None:
            raise BenchmarkFailure(
                "RenderDoc target-control helper is required for capture"
            )
        renderdoc_dir = artifact_dir / "renderdoc"
        renderdoc_dir.mkdir()
        launch_client_argv = build_renderdoc_launch_argv(
            renderdoccmd=renderdoccmd,
            capture_template=renderdoc_dir / "bbb",
            working_dir=artifact_dir,
            client_argv=client_argv,
        )

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
        "x_display": x_display,
        "window_system": "x11" if x_display is not None else "wayland",
        "screenshots": [],
        "anchor_observations": [],
        "renderdoc_captures": [],
        "renderdoc_segments": (
            sorted(renderdoc_segments) if renderdoc_segments is not None else None
        ),
    }
    server_group: ProcessGroup | None = None
    client_group: ProcessGroup | None = None
    perf_session: PerfStatSession | None = None
    timeline_session: WestonTimelineSession | None = None
    compositor_group: ProcessGroup | None = None
    compositor_output: Any = None
    try:
        if weston_path is not None:
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
                f"--renderer={weston_renderer}",
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
                "BBB_VANILLA_ROOT": str(vanilla_root),
                "RUST_LOG": (
                    "bbb_net=info,bbb_native=info,bbb_control=info,"
                    "bbb_renderer=info"
                ),
            }
        )
        if x_display is None:
            environment["WAYLAND_DISPLAY"] = wayland_display
        else:
            environment.pop("WAYLAND_DISPLAY", None)
            environment["DISPLAY"] = x_display
            environment["WINIT_UNIX_BACKEND"] = "x11"
        if extra_library_path is not None:
            inherited = environment.get("LD_LIBRARY_PATH")
            environment["LD_LIBRARY_PATH"] = (
                str(extra_library_path)
                if not inherited
                else str(extra_library_path) + os.pathsep + inherited
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
                    "argv": list(launch_client_argv),
                    "renderdoc": (
                        {
                            "command": str(renderdoccmd),
                            "command_sha256": file_digest(
                                renderdoccmd, "sha256"
                            ),
                            "control": str(renderdoc_control),
                            "control_sha256": file_digest(
                                renderdoc_control, "sha256"
                            ),
                        }
                        if renderdoccmd is not None
                        and renderdoc_control is not None
                        else None
                    ),
                    "environment": {
                        key: environment[key]
                        for key in (
                            "BBB_VANILLA_ROOT",
                            "WAYLAND_DISPLAY",
                            "DISPLAY",
                            "WINIT_UNIX_BACKEND",
                            "LD_LIBRARY_PATH",
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
                launch_client_argv,
                cwd=artifact_dir,
                stdin=subprocess.DEVNULL,
                stdout=client_log,
                stderr=subprocess.STDOUT,
                env=environment,
            )
            client_process = client_group.start()
            result["client_pid"] = client_process.pid
            client_watch_process = (
                None if renderdoccmd is not None else client_process
            )
            _wait_for_log(
                server_log_path,
                f"{BENCHMARK_USERNAME} joined the game",
                process=client_watch_process,
                timeout=timeout,
            )
            renderdoc_ident: int | None = None
            if renderdoccmd is not None:
                _wait_for_log(
                    client_log_path,
                    "Launched as ID ",
                    process=None,
                    timeout=timeout,
                )
                renderdoc_ident = parse_renderdoc_target_ident(
                    client_log_path.read_text(
                        encoding="utf-8", errors="replace"
                    )
                )
                result["renderdoc_target_ident"] = renderdoc_ident
            result["warmup_seconds"] = FIXED_WARMUP_SECONDS
            _warmup_and_start_replay(server_process, replay)
            capture_markers = replay_capture_markers(replay)
            first_capture_start = capture_markers[0]
            _wait_for_log(
                server_log_path,
                benchmark_marker_line(digest, first_capture_start),
                process=client_watch_process,
                timeout=timeout,
            )
            first_residency = _control_request(
                ("127.0.0.1", control_port), "renderer.counters"
            )
            if weston_debug_path is not None:
                timeline_session = WestonTimelineSession(
                    weston_debug_path=weston_debug_path,
                    wayland_display=wayland_display,
                    output_path=artifact_dir / "weston-timeline.jsonl",
                    surface_title=BBB_WINDOW_TITLE,
                )
                result["weston_timeline_start"] = timeline_session.start()
            if mangohud_root is not None:
                if perf_path is not None:
                    perf_session = PerfStatSession(
                        perf_path=perf_path,
                        target_pid=client_process.pid,
                        output_path=artifact_dir / "perf-stat.csv",
                    )
                    result["perf_capture_start"] = perf_session.start()
                result["mangohud_capture_start"] = mangohud_set_logging(
                    process_pid=client_process.pid, enabled=True
                )
            capture_origin = time.monotonic()
            result["marker_timeline"] = [
                marker_timeline_observation(
                    first_capture_start, elapsed_seconds=0.0
                )
            ]
            result["capture_start_residency"] = [
                {
                    "capture_segment": first_capture_start["capture_segment"],
                    "server_tick": first_capture_start["server_tick"],
                    "renderer": bbb_renderdoc_residency_evidence(
                        first_residency,
                        capture_segment=str(
                            first_capture_start["capture_segment"]
                        ),
                        renderdoc_segments=renderdoc_segments,
                    ),
                }
            ]
            for marker_frame in capture_markers[1:]:
                _wait_for_log(
                    server_log_path,
                    benchmark_marker_line(digest, marker_frame),
                    process=client_watch_process,
                    timeout=timeout,
                )
                result["marker_timeline"].append(
                    marker_timeline_observation(
                        marker_frame,
                        elapsed_seconds=time.monotonic() - capture_origin,
                    )
                )
                if marker_frame["marker"] == "capture_start":
                    renderer_counters = _control_request(
                        ("127.0.0.1", control_port), "renderer.counters"
                    )
                    result["capture_start_residency"].append(
                        {
                            "capture_segment": marker_frame["capture_segment"],
                            "server_tick": marker_frame["server_tick"],
                            "renderer": bbb_renderdoc_residency_evidence(
                                renderer_counters,
                                capture_segment=str(
                                    marker_frame["capture_segment"]
                                ),
                                renderdoc_segments=renderdoc_segments,
                            ),
                        }
                    )
                    continue
                if marker_frame["marker"] != "anchor":
                    continue
                if (
                    renderdoc_ident is not None
                    and renderdoc_control is not None
                    and (
                        renderdoc_segments is None
                        or marker_frame["capture_segment"] in renderdoc_segments
                    )
                ):
                    result["renderdoc_captures"].append(
                        _capture_renderdoc_anchor(
                            control_path=renderdoc_control,
                            capture_dir=artifact_dir / "renderdoc",
                            target_ident=renderdoc_ident,
                            client="bbb",
                            segment=marker_frame["capture_segment"],
                            server_tick=marker_frame["server_tick"],
                        )
                    )
                time.sleep(0.5)
                renderer_counters = _control_request(
                    ("127.0.0.1", control_port), "renderer.counters"
                )
                bbb_renderdoc_residency_evidence(
                    renderer_counters,
                    capture_segment=str(marker_frame["capture_segment"]),
                    renderdoc_segments=renderdoc_segments,
                )
                result["anchor_observations"].append(
                    {
                        "capture_segment": marker_frame["capture_segment"],
                        "server_tick": marker_frame["server_tick"],
                        "renderer": renderer_counters,
                        "chunk_view": _control_request(
                            ("127.0.0.1", control_port), "world.chunk_view"
                        ),
                        "local_player": _control_request(
                            ("127.0.0.1", control_port),
                            "world.client_local_player",
                        ),
                    }
                )
                if weston_path is not None:
                    result["screenshots"].append(
                        _take_weston_screenshot(
                            artifact_dir=artifact_dir,
                            environment=environment,
                            client="bbb",
                            segment=marker_frame["capture_segment"],
                            tick=marker_frame["server_tick"],
                        )
                    )
            if mangohud_root is not None:
                result["mangohud_capture_stop"] = mangohud_set_logging(
                    process_pid=client_process.pid,
                    enabled=False,
                    completion_dir=artifact_dir / "mangohud",
                )
                if perf_session is not None:
                    result["perf_capture_stop"] = perf_session.stop()
            if timeline_session is not None:
                result["weston_timeline"] = timeline_session.stop()
            if mangohud_root is None:
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
        if perf_session is not None and (
            perf_session.process is not None
            and perf_session.process.poll() is None
        ):
            try:
                result["perf_capture_stop"] = perf_session.stop()
            except BenchmarkFailure as error:
                result.setdefault("invalid_reasons", []).append(str(error))
        if timeline_session is not None and (
            timeline_session.process is not None
            and timeline_session.process.poll() is None
        ):
            try:
                result["weston_timeline"] = timeline_session.stop()
            except BenchmarkFailure as error:
                result.setdefault("invalid_reasons", []).append(str(error))
        if client_group is not None:
            if mangohud_root is not None:
                result["client_cleanup"] = {
                    "signal": "SIGKILL",
                    "reason": (
                        "MangoHud 0.8.4 Vulkan reproducibly crashes during "
                        "ordered application teardown; "
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
                    marker_timeline = result.get("marker_timeline")
                    if not isinstance(marker_timeline, list):
                        raise BenchmarkFailure(
                            "bbb capture lacks an observed marker timeline"
                        )
                    mango_summary = summarize_mangohud_logs(
                        raw_logs, replay, marker_timeline
                    )
                    result["mangohud_frame_summaries"] = mango_summary["overall"]
                    result["mangohud_segment_summaries"] = mango_summary["segments"]
                    result["presented_frames"] = mango_summary["presented_frames"]
                    result["capture_api"] = "vulkan"
                    thermal = mango_summary["overall"][0]
                    peaks = (
                        thermal["cpu_temp_peak_c"],
                        thermal["gpu_temp_peak_c"],
                    )
                    result["throttling"] = (
                        None
                        if any(value is None for value in peaks)
                        else any(value >= THERMAL_REJECT_C for value in peaks)
                    )
                    result["thermal_reject_c"] = THERMAL_REJECT_C
                    perf_output = artifact_dir / "perf-stat.csv"
                    if perf_path is not None:
                        result["perf_stat"] = parse_perf_stat(
                            perf_output,
                            presented_frames=result["presented_frames"],
                        )
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
        resolve_workspace(args).root,
        java_path=Path(args.java),
        verify_asset_objects=not args.skip_asset_objects,
    )
    print(json.dumps(_json_ready(audit), indent=2, sort_keys=True))


def _command_prepare_vanilla(args: argparse.Namespace) -> None:
    vanilla_root = resolve_workspace(args).root
    artifact_dir = Path(args.artifact_dir)
    artifact_dir.mkdir(parents=True, exist_ok=True)
    audit = audit_distribution(vanilla_root, java_path=Path(args.java))
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
        assets_dir=vanilla_root / "downloads" / MINECRAFT_VERSION / "assets",
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
        vanilla_root=resolve_workspace(args).root,
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
        vanilla_root=resolve_workspace(args).root,
        java_path=Path(args.java),
        world_manifest_path=Path(args.world_manifest),
        artifact_dir=Path(args.artifact_dir),
        server_port=args.server_port or reserve_loopback_port(),
        accept_eula=args.accept_eula,
        timeout=args.timeout,
        wayland_display=args.wayland_display,
        x_display=args.x_display,
        mangohud_root=Path(args.mangohud_root) if args.mangohud_root else None,
        perf_path=Path(args.perf) if args.perf else None,
        weston_debug_path=(
            Path(args.weston_debug) if args.weston_debug else None
        ),
        weston_path=Path(args.weston) if args.weston else None,
        workload_debug_profile=args.workload_debug_profile,
        weston_renderer=args.weston_renderer,
        renderdoccmd=Path(args.renderdoccmd) if args.renderdoccmd else None,
        renderdoc_control=(
            Path(args.renderdoc_control) if args.renderdoc_control else None
        ),
        renderdoc_segments=(
            frozenset(args.renderdoc_segment) if args.renderdoc_segment else None
        ),
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
        vanilla_root=resolve_workspace(args).root,
        java_path=Path(args.java),
        bbb_binary=Path(args.bbb_binary),
        world_manifest_path=Path(args.world_manifest),
        artifact_dir=Path(args.artifact_dir),
        server_port=server_port,
        control_port=control_port,
        accept_eula=args.accept_eula,
        timeout=args.timeout,
        wayland_display=args.wayland_display,
        x_display=args.x_display,
        mangohud_root=Path(args.mangohud_root) if args.mangohud_root else None,
        perf_path=Path(args.perf) if args.perf else None,
        weston_debug_path=(
            Path(args.weston_debug) if args.weston_debug else None
        ),
        weston_path=Path(args.weston) if args.weston else None,
        weston_renderer=args.weston_renderer,
        renderdoccmd=Path(args.renderdoccmd) if args.renderdoccmd else None,
        renderdoc_control=(
            Path(args.renderdoc_control) if args.renderdoc_control else None
        ),
        extra_library_path=(
            Path(args.extra_library_path)
            if args.extra_library_path
            else None
        ),
        renderdoc_segments=(
            frozenset(args.renderdoc_segment) if args.renderdoc_segment else None
        ),
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


def _command_summarize_workload(args: argparse.Namespace) -> None:
    vanilla = load_json_object(Path(args.vanilla_smoke), "vanilla smoke")
    bbb = load_json_object(Path(args.bbb_smoke), "bbb smoke")
    vanilla_calibration = load_json_object(
        Path(args.vanilla_calibration), "Vanilla visual calibration"
    )
    bbb_calibration = load_json_object(
        Path(args.bbb_calibration), "bbb visual calibration"
    )
    bbb_provenance = load_json_object(
        Path(args.bbb_provenance), "BBB binary provenance"
    )
    vanilla_debug = (
        load_json_object(Path(args.vanilla_debug_smoke), "vanilla debug smoke")
        if args.vanilla_debug_smoke is not None
        else None
    )
    summary = summarize_workload_evidence(
        vanilla=vanilla,
        bbb=bbb,
        vanilla_calibration=vanilla_calibration,
        bbb_calibration=bbb_calibration,
        bbb_provenance=bbb_provenance,
        vanilla_debug=vanilla_debug,
        visual_review_status=args.visual_review_status,
        visual_review_note=args.visual_review_note,
        visual_artifact_dir=Path(args.visual_artifact_dir),
    )
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json_bytes(summary))
    print(output)


def _command_summarize_visual_calibration(args: argparse.Namespace) -> None:
    summary = summarize_visual_calibration(
        baseline=load_json_object(Path(args.baseline_smoke), "baseline smoke"),
        repeat=load_json_object(Path(args.repeat_smoke), "repeat smoke"),
        client=args.client,
        visual_artifact_dir=Path(args.visual_artifact_dir),
    )
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json_bytes(summary))
    print(output)


def _command_summarize_renderdoc(args: argparse.Namespace) -> None:
    manifest = load_json_object(
        Path(args.manifest), "RenderDoc evidence manifest"
    )
    summary = summarize_renderdoc_evidence(manifest)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json_bytes(summary))
    print(output)


def _command_summarize_formal(args: argparse.Namespace) -> None:
    manifest = load_json_object(
        Path(args.manifest), "formal comparison manifest"
    )
    summary = summarize_formal_comparison(manifest)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json_bytes(summary))
    print(output)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    audit = subparsers.add_parser("audit")
    add_vanilla_root(audit)
    audit.add_argument("--java", required=True)
    audit.add_argument("--skip-asset-objects", action="store_true")
    audit.set_defaults(handler=_command_audit)

    prepare = subparsers.add_parser("prepare-vanilla")
    add_vanilla_root(prepare)
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
    add_vanilla_root(prepare_world)
    prepare_world.add_argument("--java", required=True)
    prepare_world.add_argument("--artifact-dir", required=True)
    prepare_world.add_argument("--server-port", type=int)
    prepare_world.add_argument("--timeout", type=float, default=300.0)
    prepare_world.add_argument("--accept-eula", action="store_true")
    prepare_world.set_defaults(handler=_command_prepare_world)

    smoke = subparsers.add_parser("run-official-smoke")
    smoke.add_argument("--replay", required=True)
    add_vanilla_root(smoke)
    smoke.add_argument("--java", required=True)
    smoke.add_argument("--world-manifest", required=True)
    smoke.add_argument("--artifact-dir", required=True)
    smoke.add_argument("--server-port", type=int)
    smoke.add_argument("--timeout", type=float, default=240.0)
    smoke.add_argument("--accept-eula", action="store_true")
    smoke.add_argument("--wayland-display", default="bbb-gate")
    smoke.add_argument("--x-display")
    smoke.add_argument("--mangohud-root")
    smoke.add_argument("--perf")
    smoke.add_argument("--weston-debug")
    smoke.add_argument("--weston")
    smoke.add_argument("--renderdoccmd")
    smoke.add_argument("--renderdoc-control")
    smoke.add_argument(
        "--renderdoc-segment",
        action="append",
        choices=tuple(VISUAL_ROIS),
    )
    smoke.add_argument(
        "--weston-renderer",
        choices=("gl", "pixman"),
        default="gl",
    )
    smoke.add_argument(
        "--workload-debug-profile",
        action="store_true",
        help=(
            "enable Vanilla always-on chunk/entity/particle counters for "
            "validation screenshots; do not use for formal timing"
        ),
    )
    smoke.set_defaults(handler=_command_run_official_smoke)

    bbb_smoke = subparsers.add_parser("run-bbb-smoke")
    bbb_smoke.add_argument("--replay", required=True)
    add_vanilla_root(bbb_smoke)
    bbb_smoke.add_argument("--java", required=True)
    bbb_smoke.add_argument("--bbb-binary", required=True)
    bbb_smoke.add_argument("--world-manifest", required=True)
    bbb_smoke.add_argument("--artifact-dir", required=True)
    bbb_smoke.add_argument("--server-port", type=int)
    bbb_smoke.add_argument("--control-port", type=int)
    bbb_smoke.add_argument("--timeout", type=float, default=240.0)
    bbb_smoke.add_argument("--accept-eula", action="store_true")
    bbb_smoke.add_argument("--wayland-display", default="bbb-benchmark-native")
    bbb_smoke.add_argument("--x-display")
    bbb_smoke.add_argument("--mangohud-root")
    bbb_smoke.add_argument("--perf")
    bbb_smoke.add_argument("--weston-debug")
    bbb_smoke.add_argument("--weston")
    bbb_smoke.add_argument("--renderdoccmd")
    bbb_smoke.add_argument("--renderdoc-control")
    bbb_smoke.add_argument(
        "--renderdoc-segment",
        action="append",
        choices=tuple(VISUAL_ROIS),
    )
    bbb_smoke.add_argument("--extra-library-path")
    bbb_smoke.add_argument(
        "--weston-renderer",
        choices=("gl", "pixman"),
        default="gl",
    )
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

    workload = subparsers.add_parser("summarize-workload")
    workload.add_argument("--vanilla-smoke", required=True)
    workload.add_argument("--vanilla-debug-smoke")
    workload.add_argument("--bbb-smoke", required=True)
    workload.add_argument("--vanilla-calibration", required=True)
    workload.add_argument("--bbb-calibration", required=True)
    workload.add_argument("--bbb-provenance", required=True)
    workload.add_argument(
        "--visual-review-status",
        choices=("match", "mismatch"),
        required=True,
    )
    workload.add_argument("--visual-review-note", required=True)
    workload.add_argument("--visual-artifact-dir", required=True)
    workload.add_argument("--output", required=True)
    workload.set_defaults(handler=_command_summarize_workload)

    calibration = subparsers.add_parser("summarize-visual-calibration")
    calibration.add_argument("--baseline-smoke", required=True)
    calibration.add_argument("--repeat-smoke", required=True)
    calibration.add_argument("--client", choices=("vanilla", "bbb"), required=True)
    calibration.add_argument("--visual-artifact-dir", required=True)
    calibration.add_argument("--output", required=True)
    calibration.set_defaults(handler=_command_summarize_visual_calibration)

    renderdoc = subparsers.add_parser("summarize-renderdoc")
    renderdoc.add_argument("--manifest", required=True)
    renderdoc.add_argument("--output", required=True)
    renderdoc.set_defaults(handler=_command_summarize_renderdoc)

    formal = subparsers.add_parser("summarize-formal")
    formal.add_argument("--manifest", required=True)
    formal.add_argument("--output", required=True)
    formal.set_defaults(handler=_command_summarize_formal)
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
