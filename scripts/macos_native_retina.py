#!/usr/bin/env python3
"""macOS built-in-display native-drawable performance acceptance.

This is deliberately separate from ``official_client_benchmark.py``'s fixed
1280x720 Linux baseline.  ``system_profiler`` supplies the built-in panel's
physical native target while CoreGraphics records the current logical/backing
mode and scale. Screenshots attest the drawable actually rendered by each
client, and formal aggregation refuses to compare runs when those pixels differ.
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
import plistlib
import re
import shutil
import statistics
import struct
import subprocess
import sys
import time
import tomllib
import zipfile
from collections.abc import Callable, Mapping, Sequence
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

SCHEMA = 3
PROFILE_ID = "macos-native-retina"
FORMAL_ORDER = ("vanilla", "bbb", "bbb", "vanilla") * 2
FORMAL_ROUNDS = ((1, 2, 3, 4), (5, 6, 7, 8))
# Pair each BBB capture with the nearest official capture in its ABBA round.
FORMAL_RUN_PAIRS = ((1, 2), (4, 3), (5, 6), (8, 7))
CAPTURE_SEGMENTS = (
    "static_orbit",
    "preloaded_flight",
    "chunk_boundary_crossing",
)
WARMUP_SECONDS = 30
PROFILE_SECONDS = 10
UNLIMITED_FPS_SENTINEL = 260
HITCH_THRESHOLDS_MS = (33.333, 50.0, 100.0)
STABILITY_POLICY = "absolute-tail-v2"
BASELINE_BBB_COMMIT = "d698c8db0d219f035c7597726c76225c6cd70abc"
BASELINE_BBB_REPORT_SHA256 = "2864ea8d003cf53cc98ce782ecfe9d89112b5092d6786b9c221af1e634e42b9b"
BASELINE_BBB_BINARY_SHA256 = "29cfa6e4fb5d55b73a53c8ebed111f988dcb58e646be281458af9b465f68fd97"
MAX_BASELINE_FPS_MARGIN = 0.01
LEGACY_CANDIDATE_AB_SCHEMA = "macos-native-retina-bbb-ab-v1"
CANDIDATE_AB_SCHEMA = "macos-native-retina-bbb-ab-v2"
WGPU30_CANDIDATE_AB_SCHEMA = "macos-native-retina-bbb-ab-v3"
WGPU30_OPTIMIZATION_AB_SCHEMA = "macos-native-retina-bbb-ab-v4"
ACCEPTED_CANDIDATE_AB_SCHEMAS = (
    LEGACY_CANDIDATE_AB_SCHEMA,
    CANDIDATE_AB_SCHEMA,
    WGPU30_CANDIDATE_AB_SCHEMA,
    WGPU30_OPTIMIZATION_AB_SCHEMA,
)
WGPU30_OPTIMIZATION_BASELINE_COMMIT = "5b1903221acac8a2487fb5499dbf970ee192c97d"
WGPU30_OPTIMIZATION_BASELINE_REPORT_SHA256 = (
    "7a5fc89662d9da50cb6f9556c1af68f9ce46c777f8778e3f30b6d651c90bfcfa"
)
WGPU30_OPTIMIZATION_BASELINE_BINARY_SHA256 = (
    "849a9300ed1b1325137506f7519782b57c264eb31347d36725c96686afa20817"
)
WGPU30_OPTIMIZATION_BASELINE_REPEAT_FPS = (
    195.82836970695612,
    194.41391459437463,
    194.42203984309396,
    194.18012527990288,
)
WGPU30_OPTIMIZATION_BASELINE_RAW_SHA256 = (
    "6860b4a74d3edf14d6a043eb71f1ef4d0a3e4c2b746cdbaae112ce4b8308acec"
)
WGPU30_GPU_TIMESTAMP_STRATEGY = "PassBoundary"
CANDIDATE_AB_ORDER = ("baseline", "candidate", "candidate", "baseline") * 2
CANDIDATE_STABILITY_POLICY = "candidate-absolute-tail-v1"
LEGACY_FRAME_PHASE_SCHEMA = "renderer-frame-phases-v2"
LEGACY_FRAME_PHASE_COLUMNS = (
    "application_frame_interval",
    "cpu_encode_before_wait",
    "previous_submission_wait",
    "previous_submission_poll",
    "previous_submission_blocking_wait",
    "surface_acquire",
    "presentation_tail_encode",
    "queue_submit",
    "swap_buffers_present",
    "gpu_async_poll",
    "render_call_total",
)
FRAME_PHASE_SCHEMA = "renderer-frame-phases-v3"
FRAME_PHASE_COLUMNS = (
    *LEGACY_FRAME_PHASE_COLUMNS[:-2],
    "post_present_fence_submit",
    *LEGACY_FRAME_PHASE_COLUMNS[-2:],
)
FRAME_PHASE_CONTRACTS = {
    LEGACY_FRAME_PHASE_SCHEMA: LEGACY_FRAME_PHASE_COLUMNS,
    FRAME_PHASE_SCHEMA: FRAME_PHASE_COLUMNS,
}
FRAME_FLIGHT_SCHEMA = "renderer-frame-flight-v1"
FRAME_FLIGHT_DURATION_COLUMNS = (
    "render_to_callback_observed",
    "submit_to_callback_observed",
    "callback_to_wait_return",
)
FRAME_FLIGHT_COUNT_COLUMNS = (
    "in_flight_depth_at_submit",
    "completion_observation_expected",
    "completion_observation_missing",
    "completion_observations_dropped",
)
FRAME_FLIGHT_COLUMNS = FRAME_FLIGHT_DURATION_COLUMNS + FRAME_FLIGHT_COUNT_COLUMNS
GPU_STAGE_SCHEMA = "renderer-gpu-stages-v1"
GPU_STAGE_DURATION_COLUMNS = (
    "terrain_atlas_animation",
    "opaque_world",
    "opaque_particles",
    "feature_depth_copies",
    "entity_translucent_features",
    "remaining_feature_targets",
    "clouds",
    "weather",
    "transparency_combine",
    "presentation_tail",
    "encoder_tail",
)
GPU_STAGE_COLUMNS = (
    "application_frame_interval",
    "total_gpu_duration",
) + GPU_STAGE_DURATION_COLUMNS
INPUT_LATENCY_SCHEMA = "renderer-input-latency-v1"
INPUT_LATENCY_DURATION_COLUMNS = (
    "enqueue_to_simulation_apply",
    "simulation_apply_to_renderer_extract",
    "renderer_extract_to_queue_submit",
    "queue_submit_to_present_call_return",
    "enqueue_to_present_call_return",
)
INPUT_LATENCY_COUNT_COLUMNS = (
    "sequence",
    "superseded_events",
    "out_of_order_events",
    "present_call_return_software_boundary",
)
INPUT_LATENCY_COLUMNS = INPUT_LATENCY_DURATION_COLUMNS + INPUT_LATENCY_COUNT_COLUMNS
BBB_RUNTIME_LOG_FILTERS = (
    "bbb_native=info",
    "bbb_renderer=info",
    "bbb_net=info",
)
GPU_STAGE_RUNTIME_LOG_FILTER = "bbb_renderer::gpu_timing=debug"
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
WGPU_DEPENDENCY_RECEIPT_PATH = (
    REPO_ROOT / "docs" / "evidence" / "dependency-receipts" / "wgpu-30.0.0.json"
)
WGPU_COMPONENT_VERSIONS = {
    "naga": "30.0.0",
    "wgpu": "30.0.0",
    "wgpu-core": "30.0.0",
    "wgpu-hal": "30.0.0",
    "wgpu-types": "30.0.0",
}
WGPU_REGISTRY_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
WGPU_METAL_FENCE_COMMIT = "e0a6065c65b5b6c2e92ee7052ed084cd8910d486"
WGPU_METAL_FENCE_STRATEGY = "completion-handler condition-variable wait"


class RetinaAcceptanceError(RuntimeError):
    """The native-Retina contract or one of its evidence files is invalid."""


def _console_locked_from_ioreg_plist(payload: bytes) -> bool:
    """Read the kernel-owned console lock bit used by the capture preflight."""

    try:
        value = plistlib.loads(payload)
    except plistlib.InvalidFileException as error:
        raise RetinaAcceptanceError("IORegistry root is not a valid plist") from error
    if not isinstance(value, Mapping):
        raise RetinaAcceptanceError("IORegistry root plist is not a dictionary")
    locked = value.get("IOConsoleLocked")
    if not isinstance(locked, bool):
        raise RetinaAcceptanceError("IORegistry root lacks boolean IOConsoleLocked")
    return locked


def require_unlocked_macos_console() -> None:
    """Reject formal input capture before launch while loginwindow owns the seat."""

    if sys.platform != "darwin":
        raise RetinaAcceptanceError("Retina capture requires a macOS console session")
    result = subprocess.run(
        ("/usr/sbin/ioreg", "-n", "Root", "-d", "1", "-a"),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        error = result.stderr.decode("utf-8", errors="replace").strip()
        raise RetinaAcceptanceError(f"IORegistry console query failed: {error}")
    if _console_locked_from_ioreg_plist(result.stdout):
        raise RetinaAcceptanceError(
            "macOS console is locked; unlock the graphical session before Retina capture"
        )


def workspace_wgpu_dependency_receipt(repo: Path = REPO_ROOT) -> dict[str, object]:
    cargo_toml_path = repo / "Cargo.toml"
    cargo_lock_path = repo / "Cargo.lock"
    try:
        cargo_toml_bytes = cargo_toml_path.read_bytes()
        cargo_lock_bytes = cargo_lock_path.read_bytes()
        manifest = tomllib.loads(cargo_toml_bytes.decode("utf-8"))
        lock = tomllib.loads(cargo_lock_bytes.decode("utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise RetinaAcceptanceError(f"cannot read Cargo dependency receipt: {error}") from error

    requirement = manifest.get("workspace", {}).get("dependencies", {}).get("wgpu")
    if requirement != "30.0.0":
        raise RetinaAcceptanceError(
            f"workspace wgpu dependency must be exactly 30.0.0, got {requirement!r}"
        )
    packages = lock.get("package")
    if not isinstance(packages, list):
        raise RetinaAcceptanceError("Cargo.lock lacks package receipts")
    resolved: dict[str, dict[str, str]] = {}
    for name, expected_version in WGPU_COMPONENT_VERSIONS.items():
        matches = [entry for entry in packages if entry.get("name") == name]
        if len(matches) != 1:
            raise RetinaAcceptanceError(
                f"Cargo.lock must resolve exactly one {name}, got {len(matches)}"
            )
        entry = matches[0]
        version = entry.get("version")
        source = entry.get("source")
        checksum = entry.get("checksum")
        if version != expected_version or source != WGPU_REGISTRY_SOURCE:
            raise RetinaAcceptanceError(
                f"Cargo.lock resolved {name} {version!r} from {source!r}, "
                f"expected registry {expected_version}"
            )
        if not isinstance(checksum, str) or re.fullmatch(r"[0-9a-f]{64}", checksum) is None:
            raise RetinaAcceptanceError(f"Cargo.lock {name} checksum is missing")
        resolved[name] = {"version": version, "checksum": checksum}

    patch = manifest.get("patch", {}).get("crates-io", {})
    local_patch = isinstance(patch, Mapping) and "wgpu-hal" in patch
    manifest_paths = [cargo_toml_path, *sorted((repo / "crates").glob("*/Cargo.toml"))]
    direct_hal = any(
        re.search(r"(?m)^\s*wgpu-hal\s*=", path.read_text(encoding="utf-8"))
        for path in manifest_paths
    )
    vendored_hal = (repo / "third_party" / "wgpu-hal-0.19.5").exists()
    if local_patch or direct_hal or vendored_hal:
        raise RetinaAcceptanceError(
            "wgpu 30 receipt forbids patched, direct, or vendored wgpu-hal"
        )
    return {
        "schema": 1,
        "workspace_dependency": "wgpu = 30.0.0",
        "cargo_toml_sha256": hashlib.sha256(cargo_toml_bytes).hexdigest(),
        "cargo_lock_sha256": hashlib.sha256(cargo_lock_bytes).hexdigest(),
        "registry_source": WGPU_REGISTRY_SOURCE,
        "packages": resolved,
        "local_wgpu_hal_patch": False,
        "vendored_wgpu_hal": False,
        "direct_wgpu_hal_dependency": False,
        "upstream_metal_fence_strategy": {
            "first_release": "wgpu 30.0.0",
            "commit": WGPU_METAL_FENCE_COMMIT,
            "title": "fix(metal): Wait using a condition variable instead of polling",
            "strategy": WGPU_METAL_FENCE_STRATEGY,
            "fixes_issue": 8119,
            "commit_url": (
                "https://github.com/gfx-rs/wgpu/commit/"
                f"{WGPU_METAL_FENCE_COMMIT}"
            ),
            "release_url": "https://github.com/gfx-rs/wgpu/releases/tag/v30.0.0",
        },
    }


def load_committed_wgpu_dependency_receipt(
    path: Path = WGPU_DEPENDENCY_RECEIPT_PATH,
    repo: Path = REPO_ROOT,
) -> dict[str, object]:
    try:
        committed = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RetinaAcceptanceError(f"cannot read wgpu dependency receipt {path}: {error}") from error
    actual = workspace_wgpu_dependency_receipt(repo)
    if committed != actual:
        raise RetinaAcceptanceError("committed wgpu dependency receipt differs from Cargo files")
    return committed


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
        "frame_phase_diagnostics": {
            "schema": FRAME_PHASE_SCHEMA,
            "archive_path": "client/metrics/frame_phases.csv",
            "units": "nanoseconds",
            "columns": list(FRAME_PHASE_COLUMNS),
            "required_client": "bbb",
            "correlation_target": "application_frame_interval",
            "primary_correlation": "phase[n]-to-interval[n+1]",
            "phase_to_interval_lag_frames": 1,
        },
        "frame_flight_diagnostics": {
            "schema": FRAME_FLIGHT_SCHEMA,
            "archive_path": "client/metrics/frame_flight.csv",
            "duration_units": "nanoseconds",
            "count_units": "count",
            "duration_columns": list(FRAME_FLIGHT_DURATION_COLUMNS),
            "count_columns": list(FRAME_FLIGHT_COUNT_COLUMNS),
            "required_client": "bbb-when-run-declares-required-schema",
            "timestamp_semantics": "wgpu-callback-observation-not-hardware-completion",
            "correlation_target": "application_frame_interval",
            "primary_correlation": "observation[n]-to-interval[n+1]",
            "phase_to_interval_lag_frames": 1,
        },
        "gpu_stage_diagnostics": {
            "schema": GPU_STAGE_SCHEMA,
            "archive_path": "client/metrics/gpu_stages.csv",
            "units": "nanoseconds",
            "columns": list(GPU_STAGE_COLUMNS),
            "required_client": "bbb-when-run-declares-required-schema",
            "activation": f"RUST_LOG={GPU_STAGE_RUNTIME_LOG_FILTER}",
            "correlation_target": "same-submission-application-frame-interval",
            "total_duration_check": "ordered-stage-sum-within-rounding-tolerance",
            "timestamp_strategy": WGPU30_GPU_TIMESTAMP_STRATEGY,
        },
        "input_latency_diagnostics": {
            "schema": INPUT_LATENCY_SCHEMA,
            "archive_path": "client/metrics/input_latency.csv",
            "duration_units": "nanoseconds",
            "count_units": "count",
            "duration_columns": list(INPUT_LATENCY_DURATION_COLUMNS),
            "count_columns": list(INPUT_LATENCY_COUNT_COLUMNS),
            "required_client": "bbb-when-run-declares-required-schema",
            "activation": "--renderer-input-latency-diagnostics",
            "terminal_boundary": "present-call-return-software-boundary-not-display-or-photon",
            "coalescing": "newest-event-with-explicit-superseded-count",
        },
        "stability_acceptance": {
            "policy": STABILITY_POLICY,
            "cv_role": "diagnostic-only-when-absolute-metrics-pass",
            "required_scopes": ["combined", "scene", "abba_round", "run_pair"],
            "abba_rounds": [list(round_runs) for round_runs in FORMAL_ROUNDS],
            "abba_run_pairs": [list(pair) for pair in FORMAL_RUN_PAIRS],
            "baseline_bbb_commit": WGPU30_OPTIMIZATION_BASELINE_COMMIT,
            "baseline_bbb_report_sha256": WGPU30_OPTIMIZATION_BASELINE_REPORT_SHA256,
            "baseline_bbb_binary_sha256": WGPU30_OPTIMIZATION_BASELINE_BINARY_SHA256,
            "baseline_repeat_raw_sha256": WGPU30_OPTIMIZATION_BASELINE_RAW_SHA256,
            "baseline_repeat_mean_fps": list(
                WGPU30_OPTIMIZATION_BASELINE_REPEAT_FPS
            ),
            "maximum_candidate_fps_margin": MAX_BASELINE_FPS_MARGIN,
        },
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
    panel_native_width: int
    panel_native_height: int
    refresh_hz: float | None
    metal_device: str

    @property
    def target_drawable(self) -> tuple[int, int]:
        return (self.panel_native_width, self.panel_native_height)


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
    if panel_native is None or min(panel_native) <= 0:
        raise RetinaAcceptanceError(
            "system_profiler did not report a positive built-in panel-native resolution"
        )

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
        panel_native_width=panel_native[0],
        panel_native_height=panel_native[1],
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


def fullscreen_video_mode(display: DisplayProbe) -> str:
    """Return Vanilla's exact preferred-mode spelling for the physical panel."""

    refresh_hz = round(display.refresh_hz) if display.refresh_hz is not None else 60
    if refresh_hz <= 0:
        raise RetinaAcceptanceError("panel-native fullscreen refresh must be positive")
    return (
        f"{display.panel_native_width}x{display.panel_native_height}"
        f"@{refresh_hz}:24"
    )


def launch_contract(
    display: DisplayProbe, *, server_port: int, control_port: int
) -> dict[str, object]:
    if not 1 <= server_port <= 65535 or not 1 <= control_port <= 65535:
        raise RetinaAcceptanceError("launch ports must be in 1..65535")
    drawable = list(display.target_drawable)
    logical = [display.logical_width, display.logical_height]
    common = {
        "profile": PROFILE_ID,
        "minecraft_version": MINECRAFT_VERSION,
        "username": BENCHMARK_USERNAME,
        "window_mode": "fullscreen",
        "logical_size": logical,
        "current_backing_pixels": [
            display.backing_pixel_width,
            display.backing_pixel_height,
        ],
        "target_drawable_pixels": drawable,
        "fullscreen_video_mode": fullscreen_video_mode(display),
        "view_distance_chunks": VIEW_DISTANCE,
        "resource_packs": ["vanilla"],
        "vsync": False,
        "framerate_limit": "unlimited",
        "framerate_limit_sentinel": UNLIMITED_FPS_SENTINEL,
        "warmup_seconds": WARMUP_SECONDS,
        "profile_seconds": PROFILE_SECONDS,
        "capture_segments": list(CAPTURE_SEGMENTS),
        "input_script": [
            "at each capture_start, post to exact client pid: F2 screenshot, then hold F3 and tap L",
        ],
    }
    official = (
        "--width",
        str(display.logical_width),
        "--height",
        str(display.logical_height),
        "--fullscreenWidth",
        str(display.panel_native_width),
        "--fullscreenHeight",
        str(display.panel_native_height),
        "--fullscreen",
        "--quickPlayMultiplayer",
        f"127.0.0.1:{server_port}",
    )
    bbb = (
        "--width",
        str(display.panel_native_width),
        "--height",
        str(display.panel_native_height),
        "--client-fullscreen-resolution",
        fullscreen_video_mode(display),
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


def png_file_complete(path: Path) -> bool:
    try:
        payload = path.read_bytes()
    except OSError:
        return False
    return (
        len(payload) >= 33
        and payload.startswith(PNG_SIGNATURE)
        and payload[-12:-8] == b"\x00\x00\x00\x00"
        and payload[-8:-4] == b"IEND"
    )


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


def _frame_phase_contract_from_archive(
    archive: zipfile.ZipFile,
) -> tuple[str, tuple[str, ...]] | None:
    names = [
        name for name in archive.namelist() if name.endswith("/metrics/frame_phases.csv")
    ]
    if not names:
        return None
    if len(names) != 1:
        raise RetinaAcceptanceError(
            "profiling archive must contain at most one frame_phases.csv"
        )
    text = archive.read(names[0]).decode("utf-8")
    try:
        header = next(csv.reader(text.splitlines()))
    except (StopIteration, csv.Error) as error:
        raise RetinaAcceptanceError("frame phase CSV header is malformed") from error
    if not header or header[0] != "@tick" or len(header) != len(set(header)):
        raise RetinaAcceptanceError("frame phase CSV header is malformed")
    observed = set(header[1:])
    for schema, columns in FRAME_PHASE_CONTRACTS.items():
        if observed == set(columns):
            return schema, columns
    raise RetinaAcceptanceError(
        f"frame phase columns do not match a supported schema: {sorted(observed)}"
    )


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
    stddev_ms = statistics.pstdev(ordered) if len(ordered) > 1 else 0.0
    p50_ms = percentile(ordered, 50)
    p99_ms = percentile(ordered, 99)
    return {
        "samples": len(values),
        "mean_ms": mean_ms,
        "p50_ms": p50_ms,
        "p95_ms": percentile(ordered, 95),
        "p99_ms": p99_ms,
        "stddev_ms": stddev_ms,
        "p99_minus_p50_ms": p99_ms - p50_ms,
        "one_percent_low_fps": 1000.0 / slow_mean,
        "mean_fps": 1000.0 / mean_ms,
        "coefficient_of_variation": (
            stddev_ms / mean_ms if len(ordered) > 1 else 0.0
        ),
        "hitch_counts": {
            f">={threshold:.3f}ms": sum(value >= threshold for value in ordered)
            for threshold in HITCH_THRESHOLDS_MS
        },
    }


def pearson_correlation(left: Sequence[float], right: Sequence[float]) -> float:
    if len(left) != len(right) or len(left) < 2:
        return 0.0
    left_mean = statistics.fmean(left)
    right_mean = statistics.fmean(right)
    covariance = sum(
        (left_value - left_mean) * (right_value - right_mean)
        for left_value, right_value in zip(left, right, strict=True)
    )
    left_square_sum = sum((value - left_mean) ** 2 for value in left)
    right_square_sum = sum((value - right_mean) ** 2 for value in right)
    divisor = math.sqrt(left_square_sum * right_square_sum)
    return covariance / divisor if divisor else 0.0


def summarize_frame_phase_ms(
    values: Sequence[float],
    application_intervals: Sequence[float],
    *,
    next_phase_values: Sequence[float] | None = None,
    next_application_intervals: Sequence[float] | None = None,
) -> dict[str, object]:
    if (
        not values
        or len(values) != len(application_intervals)
        or any(not math.isfinite(value) or value < 0 for value in values)
        or any(not math.isfinite(value) or value <= 0 for value in application_intervals)
    ):
        raise RetinaAcceptanceError(
            "frame phase samples must be finite, non-negative, and paired with positive intervals"
        )
    if next_phase_values is None and next_application_intervals is None:
        next_phase_values = values[:-1]
        next_application_intervals = application_intervals[1:]
    if (
        next_phase_values is None
        or next_application_intervals is None
        or len(next_phase_values) != len(next_application_intervals)
        or any(not math.isfinite(value) or value < 0 for value in next_phase_values)
        or any(
            not math.isfinite(value) or value <= 0
            for value in next_application_intervals
        )
    ):
        raise RetinaAcceptanceError(
            "next-frame correlation samples must be finite, paired, and temporally aligned"
        )
    ordered = sorted(values)
    return {
        "samples": len(values),
        "mean_ms": statistics.fmean(values),
        "p50_ms": percentile(ordered, 50),
        "p95_ms": percentile(ordered, 95),
        "p99_ms": percentile(ordered, 99),
        "stddev_ms": statistics.pstdev(values) if len(values) > 1 else 0.0,
        "prior_application_frame_interval_correlation": pearson_correlation(
            values, application_intervals
        ),
        "next_application_frame_interval_correlation": pearson_correlation(
            next_phase_values, next_application_intervals
        ),
    }


def summarize_frame_flight(
    samples: Mapping[str, Sequence[float]],
    application_intervals: Sequence[float],
    *,
    next_samples: Mapping[str, Sequence[float]] | None = None,
    next_application_intervals: Sequence[float] | None = None,
) -> dict[str, object]:
    if set(samples) != set(FRAME_FLIGHT_COLUMNS):
        raise RetinaAcceptanceError("frame flight columns differ from the committed schema")
    if next_samples is None and next_application_intervals is None:
        next_samples = {
            column: values[:-1] for column, values in samples.items()
        }
        next_application_intervals = application_intervals[1:]
    if (
        next_samples is None
        or next_application_intervals is None
        or set(next_samples) != set(FRAME_FLIGHT_COLUMNS)
        or any(len(values) != len(application_intervals) for values in samples.values())
        or any(
            len(values) != len(next_application_intervals)
            for values in next_samples.values()
        )
    ):
        raise RetinaAcceptanceError("frame flight next-frame samples are malformed")
    observed_indexes = [
        index
        for index, (expected, missing) in enumerate(
            zip(
                samples["completion_observation_expected"],
                samples["completion_observation_missing"],
                strict=True,
            )
        )
        if expected == 1.0 and missing == 0.0
    ]
    observed_next_indexes = [
        index
        for index, (expected, missing) in enumerate(
            zip(
                next_samples["completion_observation_expected"],
                next_samples["completion_observation_missing"],
                strict=True,
            )
        )
        if expected == 1.0 and missing == 0.0
    ]
    if not observed_indexes:
        raise RetinaAcceptanceError("frame flight archive contains no callback observation")
    durations = {
        column: summarize_frame_phase_ms(
            [values[index] for index in observed_indexes],
            [application_intervals[index] for index in observed_indexes],
            next_phase_values=[
                next_samples[column][index] for index in observed_next_indexes
            ],
            next_application_intervals=[
                next_application_intervals[index] for index in observed_next_indexes
            ],
        )
        for column, values in samples.items()
        if column in FRAME_FLIGHT_DURATION_COLUMNS
    }
    counts: dict[str, object] = {}
    for column in FRAME_FLIGHT_COUNT_COLUMNS:
        values = samples[column]
        if (
            not values
            or len(values) != len(application_intervals)
            or any(not math.isfinite(value) or value < 0 for value in values)
        ):
            raise RetinaAcceptanceError(
                "frame flight count samples must be finite, non-negative, and aligned"
            )
        ordered = sorted(values)
        counts[column] = {
            "samples": len(values),
            "mean": statistics.fmean(values),
            "p50": percentile(ordered, 50),
            "p95": percentile(ordered, 95),
            "p99": percentile(ordered, 99),
            "stddev": statistics.pstdev(values) if len(values) > 1 else 0.0,
            "minimum": ordered[0],
            "maximum": ordered[-1],
            "total": sum(values),
            "prior_application_frame_interval_correlation": pearson_correlation(
                values, application_intervals
            ),
            "next_application_frame_interval_correlation": pearson_correlation(
                next_samples[column], next_application_intervals
            ),
        }
    return {"durations": durations, "counts": counts}


def summarize_gpu_stages(
    samples: Mapping[str, Sequence[float]],
) -> dict[str, object]:
    if set(samples) != set(GPU_STAGE_COLUMNS):
        raise RetinaAcceptanceError("GPU stage columns differ from the committed schema")
    application_intervals = samples["application_frame_interval"]
    total_durations = samples["total_gpu_duration"]
    sample_count = len(application_intervals)
    if (
        sample_count == 0
        or len(total_durations) != sample_count
        or any(
            len(samples[column]) != sample_count
            for column in GPU_STAGE_DURATION_COLUMNS
        )
        or any(
            not math.isfinite(value) or value <= 0.0
            for value in (*application_intervals, *total_durations)
        )
        or any(
            not math.isfinite(value) or value < 0.0
            for column in GPU_STAGE_DURATION_COLUMNS
            for value in samples[column]
        )
    ):
        raise RetinaAcceptanceError(
            "GPU stage samples must be finite, aligned, and use positive frame/total durations"
        )
    # Rust rounds each of eleven adjacent timestamp deltas independently and
    # rounds the endpoint total once. Their sums may therefore differ by at
    # most one nanosecond per stage.
    rounding_tolerance_ms = len(GPU_STAGE_DURATION_COLUMNS) / 1_000_000.0
    for index, total in enumerate(total_durations):
        stage_sum = sum(samples[column][index] for column in GPU_STAGE_DURATION_COLUMNS)
        if abs(stage_sum - total) > rounding_tolerance_ms:
            raise RetinaAcceptanceError(
                "GPU stage sum differs from the same-submission total duration"
            )

    stages: dict[str, object] = {}
    for column in GPU_STAGE_DURATION_COLUMNS:
        values = samples[column]
        ordered = sorted(values)
        mean_ms = statistics.fmean(values)
        p50_ms = percentile(ordered, 50)
        p99_ms = percentile(ordered, 99)
        stages[column] = {
            "samples": sample_count,
            "mean_ms": mean_ms,
            "p50_ms": p50_ms,
            "p95_ms": percentile(ordered, 95),
            "p99_ms": p99_ms,
            "stddev_ms": statistics.pstdev(values) if sample_count > 1 else 0.0,
            "p99_minus_p50_ms": p99_ms - p50_ms,
            "mean_total_gpu_share": mean_ms / statistics.fmean(total_durations),
            "total_gpu_duration_correlation": pearson_correlation(
                values, total_durations
            ),
            "application_frame_interval_correlation": pearson_correlation(
                values, application_intervals
            ),
        }
    return {
        "total_gpu_duration": summarize_durations_ms(total_durations),
        "application_frame_interval": summarize_durations_ms(application_intervals),
        "stages": stages,
    }


def summarize_input_latency(samples: Mapping[str, Sequence[float]]) -> dict[str, object]:
    if set(samples) != set(INPUT_LATENCY_COLUMNS):
        raise RetinaAcceptanceError("input latency columns differ from the declared schema")
    lengths = {len(values) for values in samples.values()}
    if len(lengths) != 1 or not lengths or next(iter(lengths)) == 0:
        raise RetinaAcceptanceError("input latency columns are empty or unaligned")
    sequences = samples["sequence"]
    if any(
        not math.isfinite(value) or value <= 0.0 or not value.is_integer()
        for value in sequences
    ):
        raise RetinaAcceptanceError("input latency sequences must be positive integers")
    if any(current <= previous for previous, current in zip(sequences, sequences[1:])):
        raise RetinaAcceptanceError("input latency sequences are not strictly increasing")
    for column in ("superseded_events", "out_of_order_events"):
        if any(
            not math.isfinite(value) or value < 0.0 or not value.is_integer()
            for value in samples[column]
        ):
            raise RetinaAcceptanceError(
                f"input latency {column} must contain non-negative integers"
            )
    if any(value != 1.0 for value in samples["present_call_return_software_boundary"]):
        raise RetinaAcceptanceError(
            "input latency terminal boundary is not marked as software present-call return"
        )
    return {
        "sample_count": len(sequences),
        "first_sequence": int(sequences[0]),
        "last_sequence": int(sequences[-1]),
        "superseded_events": int(sum(samples["superseded_events"])),
        "out_of_order_events": int(sum(samples["out_of_order_events"])),
        "terminal_boundary": "present-call-return-software-boundary-not-display-or-photon",
        "stages": {
            column: summarize_durations_ms(samples[column])
            for column in INPUT_LATENCY_DURATION_COLUMNS
        },
    }


def profiler_samples(path: Path) -> dict[str, object]:
    try:
        archive = zipfile.ZipFile(path)
    except (OSError, zipfile.BadZipFile) as error:
        raise RetinaAcceptanceError(f"cannot open profiling archive {path}: {error}") from error
    gpu = None
    frame_phase_schema: str | None = None
    frame_phase_rows: dict[str, dict[int, float]] | None = None
    frame_flight_rows: dict[str, dict[int, float]] | None = None
    gpu_stage_rows: dict[str, dict[int, float]] | None = None
    input_latency_rows: dict[str, dict[int, float]] | None = None
    with archive:
        ticking = _read_metric_csv(archive, "/metrics/ticking.csv", "ticktime")
        try:
            gpu_duration = _read_metric_csv(archive, "/metrics/gpu.csv", "gpuDuration")
        except RetinaAcceptanceError:
            gpu_duration = None
            gpu = _read_metric_csv(archive, "/metrics/gpu.csv", "gpuUtilization")
        frame_phase_contract = _frame_phase_contract_from_archive(archive)
        if frame_phase_contract is not None:
            frame_phase_schema, frame_phase_columns = frame_phase_contract
            frame_phase_rows = {
                column: _read_metric_csv(
                    archive, "/metrics/frame_phases.csv", column
                )
                for column in frame_phase_columns
            }
        flight_names = [
            name
            for name in archive.namelist()
            if name.endswith("/metrics/frame_flight.csv")
        ]
        if flight_names:
            if len(flight_names) != 1:
                raise RetinaAcceptanceError(
                    "profiling archive must contain at most one frame_flight.csv"
                )
            frame_flight_rows = {
                column: _read_metric_csv(
                    archive, "/metrics/frame_flight.csv", column
                )
                for column in FRAME_FLIGHT_COLUMNS
            }
        gpu_stage_names = [
            name
            for name in archive.namelist()
            if name.endswith("/metrics/gpu_stages.csv")
        ]
        if gpu_stage_names:
            if len(gpu_stage_names) != 1:
                raise RetinaAcceptanceError(
                    "profiling archive must contain at most one gpu_stages.csv"
                )
            gpu_stage_rows = {
                column: _read_metric_csv(
                    archive, "/metrics/gpu_stages.csv", column
                )
                for column in GPU_STAGE_COLUMNS
            }
        input_latency_names = [
            name
            for name in archive.namelist()
            if name.endswith("/metrics/input_latency.csv")
        ]
        if input_latency_names:
            if len(input_latency_names) != 1:
                raise RetinaAcceptanceError(
                    "profiling archive must contain at most one input_latency.csv"
                )
            input_latency_rows = {
                column: _read_metric_csv(
                    archive, "/metrics/input_latency.csv", column
                )
                for column in INPUT_LATENCY_COLUMNS
            }
    if gpu_duration is not None:
        frame_ms = [ticking[tick] / 1_000_000.0 for tick in sorted(ticking)]
        gpu_ms = [
            gpu_duration[tick] / 1_000_000.0
            for tick in sorted(gpu_duration)
            if gpu_duration[tick] > 0.0
        ]
        gpu_duration_scope = (
            "renderer timestamp-query pair read directly from one completed GPU "
            "submission; no asynchronous utilization/frame-time reconstruction"
        )
    else:
        assert gpu is not None
        common_ticks = sorted(ticking.keys() & gpu.keys())
        if not common_ticks:
            raise RetinaAcceptanceError("profiling ticktime and GPU samples do not overlap")
        frame_ms = [ticking[tick] / 1_000_000.0 for tick in common_ticks]
        gpu_ms = [
            ticking[tick] * gpu[tick] / 100.0 / 1_000_000.0
            for tick in common_ticks
            if gpu[tick] > 0.0
        ]
        gpu_duration_scope = (
            "unmodified official 26.1 F3+L: Minecraft divides TimerQuery duration "
            "by that query's saved CPU frame duration, while the archive exports "
            "only sampled ticktime and gpuUtilization; duration is reconstructed "
            "as exported ticktime * gpuUtilization / 100"
        )
    if not gpu_ms:
        raise RetinaAcceptanceError("profiling archive contains no positive GPU timer sample")
    result: dict[str, object] = {
        "application_frame_time": frame_ms,
        "gpu_duration": gpu_ms,
        "gpu_duration_scope": gpu_duration_scope,
    }
    if frame_phase_rows is not None:
        assert frame_phase_schema is not None
        expected_ticks = set(ticking)
        for column, rows in frame_phase_rows.items():
            if set(rows) != expected_ticks:
                raise RetinaAcceptanceError(
                    f"frame phase {column} ticks differ from application frames"
                )
        interval = frame_phase_rows["application_frame_interval"]
        if any(interval[tick] != ticking[tick] for tick in expected_ticks):
            raise RetinaAcceptanceError(
                "frame phase application interval differs from profiler ticktime"
            )
        total_sync = frame_phase_rows["previous_submission_wait"]
        completion_poll = frame_phase_rows["previous_submission_poll"]
        blocking_wait = frame_phase_rows["previous_submission_blocking_wait"]
        if any(
            completion_poll[tick] + blocking_wait[tick] > total_sync[tick]
            for tick in expected_ticks
        ):
            raise RetinaAcceptanceError(
                "frame phase submission poll plus blocking wait exceeds total synchronization"
            )
        ordered_ticks = sorted(expected_ticks)
        result["frame_phases"] = {
            column: [rows[tick] / 1_000_000.0 for tick in ordered_ticks]
            for column, rows in frame_phase_rows.items()
        }
        result["frame_phase_schema"] = frame_phase_schema
    if frame_flight_rows is not None:
        if frame_phase_rows is None:
            raise RetinaAcceptanceError(
                "frame flight evidence requires aligned frame phase evidence"
            )
        expected_ticks = set(ticking)
        for column, rows in frame_flight_rows.items():
            if set(rows) != expected_ticks:
                raise RetinaAcceptanceError(
                    f"frame flight {column} ticks differ from application frames"
                )
        for tick in expected_ticks:
            render_age = frame_flight_rows["render_to_callback_observed"][tick]
            submit_age = frame_flight_rows["submit_to_callback_observed"][tick]
            callback_tail = frame_flight_rows["callback_to_wait_return"][tick]
            expected = frame_flight_rows["completion_observation_expected"][tick]
            missing = frame_flight_rows["completion_observation_missing"][tick]
            depth = frame_flight_rows["in_flight_depth_at_submit"][tick]
            dropped = frame_flight_rows["completion_observations_dropped"][tick]
            durations = (render_age, submit_age, callback_tail)
            if any(not math.isfinite(value) or value < 0.0 for value in durations):
                raise RetinaAcceptanceError(
                    "frame flight observation durations must be finite and non-negative"
                )
            if render_age < submit_age:
                raise RetinaAcceptanceError(
                    "frame flight render-to-callback age precedes submit-to-callback age"
                )
            if expected not in (0.0, 1.0) or missing not in (0.0, 1.0):
                raise RetinaAcceptanceError(
                    "frame flight expected/missing flags must be zero or one"
                )
            if missing > expected:
                raise RetinaAcceptanceError(
                    "frame flight missing observation was not expected"
                )
            if expected == 0.0 and any(value != 0.0 for value in durations):
                raise RetinaAcceptanceError(
                    "frame flight unexpected callback carries an observation age"
                )
            if missing == 1.0 and any(value != 0.0 for value in durations):
                raise RetinaAcceptanceError(
                    "frame flight missing callback carries an observation age"
                )
            if expected == 1.0 and missing == 0.0 and (
                render_age <= 0.0 or submit_age <= 0.0
            ):
                raise RetinaAcceptanceError(
                    "frame flight delivered callback lacks a positive observation age"
                )
            if depth != 1.0:
                raise RetinaAcceptanceError(
                    "frame flight macOS Immediate in-flight depth must remain one"
                )
            if dropped < 0.0 or not dropped.is_integer():
                raise RetinaAcceptanceError(
                    "frame flight dropped observations must be a non-negative integer"
                )
            if missing != 0.0 or dropped != 0.0:
                raise RetinaAcceptanceError(
                    "frame flight completion observation is missing or dropped"
                )
        ordered_ticks = sorted(expected_ticks)
        result["frame_flight"] = {
            column: [
                rows[tick] / 1_000_000.0
                if column in FRAME_FLIGHT_DURATION_COLUMNS
                else rows[tick]
                for tick in ordered_ticks
            ]
            for column, rows in frame_flight_rows.items()
        }
    if gpu_stage_rows is not None:
        tick_sets = {frozenset(rows) for rows in gpu_stage_rows.values()}
        if len(tick_sets) != 1:
            raise RetinaAcceptanceError("GPU stage columns use different profiler ticks")
        all_stage_ticks = next(iter(tick_sets))
        if gpu_duration is None:
            raise RetinaAcceptanceError(
                "GPU stage evidence requires direct gpuDuration samples"
            )
        stage_ticks = frozenset(
            tick
            for tick in all_stage_ticks
            if gpu_stage_rows["total_gpu_duration"][tick] > 0.0
        )
        if any(
            gpu_stage_rows[column][tick] != 0.0
            for tick in all_stage_ticks - stage_ticks
            for column in GPU_STAGE_COLUMNS
        ):
            raise RetinaAcceptanceError(
                "GPU stage zero-duration placeholder contains non-zero values"
            )
        duration_ticks = frozenset(
            tick for tick, value in gpu_duration.items() if value > 0.0
        )
        if stage_ticks != duration_ticks or any(
            gpu_stage_rows["total_gpu_duration"][tick] != gpu_duration[tick]
            for tick in stage_ticks
        ):
            raise RetinaAcceptanceError(
                "GPU stage total differs from the same profiler gpuDuration sample"
            )
        ordered_ticks = sorted(stage_ticks)
        gpu_stages = {
            column: [rows[tick] / 1_000_000.0 for tick in ordered_ticks]
            for column, rows in gpu_stage_rows.items()
        }
        summarize_gpu_stages(gpu_stages)
        result["gpu_stages"] = gpu_stages
    if input_latency_rows is not None:
        tick_sets = {frozenset(rows) for rows in input_latency_rows.values()}
        if len(tick_sets) != 1 or next(iter(tick_sets)) != frozenset(ticking):
            raise RetinaAcceptanceError(
                "input latency columns must align with profiler application frames"
            )
        sample_ticks = [
            tick
            for tick in sorted(ticking)
            if input_latency_rows["sequence"][tick] > 0.0
        ]
        placeholder_ticks = set(ticking) - set(sample_ticks)
        if any(
            input_latency_rows[column][tick] != 0.0
            for tick in placeholder_ticks
            for column in INPUT_LATENCY_COLUMNS
        ):
            raise RetinaAcceptanceError(
                "input latency zero-sequence placeholder contains non-zero values"
            )
        input_latency = {
            column: [
                input_latency_rows[column][tick] / 1_000_000.0
                if column in INPUT_LATENCY_DURATION_COLUMNS
                else input_latency_rows[column][tick]
                for tick in sample_ticks
            ]
            for column in INPUT_LATENCY_COLUMNS
        }
        for index in range(len(sample_ticks)):
            stage_sum = sum(
                input_latency[column][index]
                for column in INPUT_LATENCY_DURATION_COLUMNS[:-1]
            )
            total = input_latency["enqueue_to_present_call_return"][index]
            if abs(stage_sum - total) > 0.000001:
                raise RetinaAcceptanceError(
                    "input latency stage sum differs from end-to-end software duration"
                )
        summarize_input_latency(input_latency)
        result["input_latency"] = input_latency
    return result


def parse_profiler_archive(path: Path) -> dict[str, object]:
    samples = profiler_samples(path)
    parsed: dict[str, object] = {
        "path": str(path),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "application_frame_time": summarize_durations_ms(samples["application_frame_time"]),
        "gpu_duration": summarize_durations_ms(samples["gpu_duration"]),
        "gpu_duration_scope": samples["gpu_duration_scope"],
    }
    frame_phases = samples.get("frame_phases")
    if isinstance(frame_phases, Mapping):
        application_intervals = frame_phases["application_frame_interval"]
        parsed["frame_phase_schema"] = samples["frame_phase_schema"]
        parsed["frame_phases"] = {
            phase: summarize_frame_phase_ms(values, application_intervals)
            for phase, values in frame_phases.items()
        }
    frame_flight = samples.get("frame_flight")
    if isinstance(frame_flight, Mapping):
        frame_phases = samples.get("frame_phases")
        if not isinstance(frame_phases, Mapping):
            raise RetinaAcceptanceError(
                "frame flight evidence requires application frame intervals"
            )
        application_intervals = frame_phases["application_frame_interval"]
        parsed["frame_flight_schema"] = FRAME_FLIGHT_SCHEMA
        parsed["frame_flight"] = summarize_frame_flight(
            frame_flight, application_intervals
        )
    gpu_stages = samples.get("gpu_stages")
    if isinstance(gpu_stages, Mapping):
        parsed["gpu_stage_schema"] = GPU_STAGE_SCHEMA
        parsed["gpu_stages"] = summarize_gpu_stages(gpu_stages)
    input_latency = samples.get("input_latency")
    if isinstance(input_latency, Mapping):
        parsed["input_latency_schema"] = INPUT_LATENCY_SCHEMA
        parsed["input_latency"] = summarize_input_latency(input_latency)
    return parsed


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


def _same_number(actual: object, expected: float) -> bool:
    try:
        value = float(actual)
    except (TypeError, ValueError):
        return False
    return math.isclose(value, expected, rel_tol=1e-12, abs_tol=1e-12)


def _validate_stored_summary(
    stored: object, derived: Mapping[str, object], *, label: str
) -> None:
    if not isinstance(stored, Mapping):
        raise RetinaAcceptanceError(f"{label} summary is missing")
    for field, expected in derived.items():
        # Schema 1 reports do not carry the two schema 2 absolute-jitter
        # fields. All fields they do carry must still reproduce exactly from
        # the authoritative archive/raw pair.
        if field not in stored:
            continue
        actual = stored[field]
        if isinstance(expected, Mapping):
            if actual != expected:
                raise RetinaAcceptanceError(f"{label} {field} differs from raw evidence")
        elif isinstance(expected, (int, float)):
            if not _same_number(actual, float(expected)):
                raise RetinaAcceptanceError(f"{label} {field} differs from raw evidence")
        elif actual != expected:
            raise RetinaAcceptanceError(f"{label} {field} differs from raw evidence")


def _materialize_formal_samples(
    runs: Sequence[Mapping[str, object]],
) -> tuple[
    list[dict[str, object]],
    dict[tuple[int, str, str], list[float]],
    dict[tuple[int, str, str], list[float]],
    dict[tuple[int, str, str], list[float]],
    dict[tuple[int, str, str], list[float]],
    dict[tuple[int, str, str], list[float]],
]:
    """Reparse and normalize every archive exactly once.

    Raw sidecars are not trusted as an alternate source: their values must be
    byte-for-number identical to the metrics reconstructed from the hashed
    profiler archive. This permits a schema-1 formal capture to be upgraded to
    schema 2 without inventing samples.
    """

    normalized_runs: list[dict[str, object]] = []
    matrix: dict[tuple[int, str, str], list[float]] = {}
    phase_matrix: dict[tuple[int, str, str], list[float]] = {}
    flight_matrix: dict[tuple[int, str, str], list[float]] = {}
    gpu_stage_matrix: dict[tuple[int, str, str], list[float]] = {}
    input_latency_matrix: dict[tuple[int, str, str], list[float]] = {}
    for run_number, run in enumerate(runs, start=1):
        profiles = run.get("profiles")
        if not isinstance(profiles, list):
            raise RetinaAcceptanceError(f"run {run_number} lacks profiler archives")
        by_segment = {
            str(profile.get("segment")): profile
            for profile in profiles
            if isinstance(profile, Mapping)
        }
        if set(by_segment) != set(CAPTURE_SEGMENTS) or len(by_segment) != len(profiles):
            raise RetinaAcceptanceError(f"run {run_number} profiler segments are malformed")
        normalized_profiles: list[dict[str, object]] = []
        for segment in CAPTURE_SEGMENTS:
            profile = by_segment[segment]
            path_value = profile.get("path")
            if not isinstance(path_value, str):
                raise RetinaAcceptanceError(
                    f"run {run_number} {segment} profiler path is malformed"
                )
            path = Path(path_value)
            archive_sha256 = file_digest(path, "sha256")
            if profile.get("sha256") != archive_sha256:
                raise RetinaAcceptanceError(
                    f"run {run_number} {segment} profiler SHA-256 differs"
                )
            archive_samples = profiler_samples(path)
            normalized_profile = dict(profile)
            normalized_profile["sha256"] = archive_sha256
            for metric in ("application_frame_time", "gpu_duration"):
                sidecar_value = profile.get(f"{metric}_samples")
                if not isinstance(sidecar_value, str):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} lacks {metric} raw samples"
                    )
                raw = _load_object(Path(sidecar_value), f"{metric} raw samples")
                values = raw.get("milliseconds")
                if not isinstance(values, list):
                    raise RetinaAcceptanceError(f"{metric} raw samples are malformed")
                materialized = [float(value) for value in values]
                expected = archive_samples[metric]
                if materialized != expected:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} {metric} sidecar differs from archive"
                    )
                summary = summarize_durations_ms(materialized)
                _validate_stored_summary(
                    profile.get(metric),
                    summary,
                    label=f"run {run_number} {segment} {metric}",
                )
                normalized_profile[metric] = summary
                matrix[(run_number, segment, metric)] = materialized
            archive_phases = archive_samples.get("frame_phases")
            try:
                run_schema = int(run.get("capture_schema", run.get("schema", 0)))
            except (TypeError, ValueError):
                run_schema = 0
            phase_required = run.get("client") == "bbb" and run_schema >= SCHEMA
            required_phase_schema = run.get("required_frame_phase_schema")
            if (
                required_phase_schema is not None
                and required_phase_schema not in FRAME_PHASE_CONTRACTS
            ):
                raise RetinaAcceptanceError(
                    f"run {run_number} declares unknown frame phase schema"
                )
            if phase_required and not isinstance(archive_phases, Mapping):
                raise RetinaAcceptanceError(
                    f"run {run_number} {segment} lacks required "
                    f"{required_phase_schema or 'supported frame phase'} evidence"
                )
            if isinstance(archive_phases, Mapping):
                archive_phase_schema = archive_samples.get("frame_phase_schema")
                if archive_phase_schema not in FRAME_PHASE_CONTRACTS:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} frame phase schema is unknown"
                    )
                if (
                    required_phase_schema is not None
                    and archive_phase_schema != required_phase_schema
                ):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} requires {required_phase_schema} "
                        f"but archive uses {archive_phase_schema}"
                    )
                if profile.get("frame_phase_schema") != archive_phase_schema:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} frame phase schema differs"
                    )
                sidecar_value = profile.get("frame_phase_samples")
                if not isinstance(sidecar_value, str):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} lacks frame phase raw samples"
                    )
                raw = _load_object(Path(sidecar_value), "frame phase raw samples")
                raw_phases = raw.get("milliseconds")
                if not isinstance(raw_phases, Mapping):
                    raise RetinaAcceptanceError("frame phase raw samples are malformed")
                materialized_phases = {
                    str(phase): [float(value) for value in values]
                    for phase, values in raw_phases.items()
                    if isinstance(values, list)
                }
                expected_phases = {
                    str(phase): [float(value) for value in values]
                    for phase, values in archive_phases.items()
                }
                if materialized_phases != expected_phases:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} frame phase sidecar differs from archive"
                    )
                expected_phase_columns = FRAME_PHASE_CONTRACTS[archive_phase_schema]
                if set(materialized_phases) != set(expected_phase_columns):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} frame phase columns differ"
                    )
                application_intervals = materialized_phases[
                    "application_frame_interval"
                ]
                phase_summaries = {
                    phase: summarize_frame_phase_ms(values, application_intervals)
                    for phase, values in materialized_phases.items()
                }
                stored_phases = profile.get("frame_phases")
                if not isinstance(stored_phases, Mapping):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} lacks frame phase summaries"
                    )
                for phase, summary in phase_summaries.items():
                    _validate_stored_summary(
                        stored_phases.get(phase),
                        summary,
                        label=f"run {run_number} {segment} frame phase {phase}",
                    )
                    phase_matrix[(run_number, segment, phase)] = materialized_phases[
                        phase
                    ]
                normalized_profile["frame_phase_schema"] = archive_phase_schema
                normalized_profile["frame_phases"] = phase_summaries
            archive_flight = archive_samples.get("frame_flight")
            required_flight_schema = run.get("required_frame_flight_schema")
            if required_flight_schema is not None and required_flight_schema != FRAME_FLIGHT_SCHEMA:
                raise RetinaAcceptanceError(
                    f"run {run_number} declares unknown frame flight schema"
                )
            if required_flight_schema == FRAME_FLIGHT_SCHEMA and not isinstance(
                archive_flight, Mapping
            ):
                raise RetinaAcceptanceError(
                    f"run {run_number} {segment} lacks required {FRAME_FLIGHT_SCHEMA} evidence"
                )
            if isinstance(archive_flight, Mapping):
                if profile.get("frame_flight_schema") != FRAME_FLIGHT_SCHEMA:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} frame flight schema differs"
                    )
                sidecar_value = profile.get("frame_flight_samples")
                if not isinstance(sidecar_value, str):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} lacks frame flight raw samples"
                    )
                raw = _load_object(Path(sidecar_value), "frame flight raw samples")
                raw_flight = raw.get("values")
                if not isinstance(raw_flight, Mapping):
                    raise RetinaAcceptanceError("frame flight raw samples are malformed")
                materialized_flight = {
                    str(column): [float(value) for value in values]
                    for column, values in raw_flight.items()
                    if isinstance(values, list)
                }
                expected_flight = {
                    str(column): [float(value) for value in values]
                    for column, values in archive_flight.items()
                }
                if materialized_flight != expected_flight:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} frame flight sidecar differs from archive"
                    )
                if not isinstance(archive_phases, Mapping):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} frame flight lacks aligned phases"
                    )
                flight_summary = summarize_frame_flight(
                    materialized_flight,
                    archive_phases["application_frame_interval"],
                )
                _validate_stored_summary(
                    profile.get("frame_flight"),
                    flight_summary,
                    label=f"run {run_number} {segment} frame flight",
                )
                for column in FRAME_FLIGHT_COLUMNS:
                    flight_matrix[(run_number, segment, column)] = materialized_flight[
                        column
                    ]
                normalized_profile["frame_flight_schema"] = FRAME_FLIGHT_SCHEMA
                normalized_profile["frame_flight"] = flight_summary
            archive_gpu_stages = archive_samples.get("gpu_stages")
            required_gpu_stage_schema = run.get("required_gpu_stage_schema")
            if (
                required_gpu_stage_schema is not None
                and required_gpu_stage_schema != GPU_STAGE_SCHEMA
            ):
                raise RetinaAcceptanceError(
                    f"run {run_number} declares unknown GPU stage schema"
                )
            if required_gpu_stage_schema == GPU_STAGE_SCHEMA and not isinstance(
                archive_gpu_stages, Mapping
            ):
                raise RetinaAcceptanceError(
                    f"run {run_number} {segment} lacks required {GPU_STAGE_SCHEMA} evidence"
                )
            if isinstance(archive_gpu_stages, Mapping):
                if profile.get("gpu_stage_schema") != GPU_STAGE_SCHEMA:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} GPU stage schema differs"
                    )
                sidecar_value = profile.get("gpu_stage_samples")
                if not isinstance(sidecar_value, str):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} lacks GPU stage raw samples"
                    )
                raw = _load_object(Path(sidecar_value), "GPU stage raw samples")
                raw_stages = raw.get("milliseconds")
                if not isinstance(raw_stages, Mapping):
                    raise RetinaAcceptanceError("GPU stage raw samples are malformed")
                materialized_stages = {
                    str(column): [float(value) for value in values]
                    for column, values in raw_stages.items()
                    if isinstance(values, list)
                }
                expected_stages = {
                    str(column): [float(value) for value in values]
                    for column, values in archive_gpu_stages.items()
                }
                if materialized_stages != expected_stages:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} GPU stage sidecar differs from archive"
                    )
                stage_summary = summarize_gpu_stages(materialized_stages)
                _validate_stored_summary(
                    profile.get("gpu_stages"),
                    stage_summary,
                    label=f"run {run_number} {segment} GPU stages",
                )
                for column in GPU_STAGE_COLUMNS:
                    gpu_stage_matrix[(run_number, segment, column)] = materialized_stages[
                        column
                    ]
                normalized_profile["gpu_stage_schema"] = GPU_STAGE_SCHEMA
                normalized_profile["gpu_stages"] = stage_summary
            archive_input_latency = archive_samples.get("input_latency")
            required_input_latency_schema = run.get("required_input_latency_schema")
            if (
                required_input_latency_schema is not None
                and required_input_latency_schema != INPUT_LATENCY_SCHEMA
            ):
                raise RetinaAcceptanceError(
                    f"run {run_number} declares unknown input latency schema"
                )
            if required_input_latency_schema == INPUT_LATENCY_SCHEMA:
                launch_argv = run.get("launch_argv")
                if not isinstance(launch_argv, list) or (
                    "--renderer-input-latency-diagnostics" not in launch_argv
                ):
                    raise RetinaAcceptanceError(
                        f"run {run_number} requires input latency evidence without its launch flag"
                    )
            if required_input_latency_schema == INPUT_LATENCY_SCHEMA and not isinstance(
                archive_input_latency, Mapping
            ):
                raise RetinaAcceptanceError(
                    f"run {run_number} {segment} lacks required {INPUT_LATENCY_SCHEMA} evidence"
                )
            if isinstance(archive_input_latency, Mapping):
                if profile.get("input_latency_schema") != INPUT_LATENCY_SCHEMA:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} input latency schema differs"
                    )
                sidecar_value = profile.get("input_latency_samples")
                if not isinstance(sidecar_value, str):
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} lacks input latency raw samples"
                    )
                raw = _load_object(Path(sidecar_value), "input latency raw samples")
                raw_latency = raw.get("values")
                if not isinstance(raw_latency, Mapping):
                    raise RetinaAcceptanceError("input latency raw samples are malformed")
                materialized_latency = {
                    str(column): [float(value) for value in values]
                    for column, values in raw_latency.items()
                    if isinstance(values, list)
                }
                expected_latency = {
                    str(column): [float(value) for value in values]
                    for column, values in archive_input_latency.items()
                }
                if materialized_latency != expected_latency:
                    raise RetinaAcceptanceError(
                        f"run {run_number} {segment} input latency sidecar differs from archive"
                    )
                latency_summary = summarize_input_latency(materialized_latency)
                _validate_stored_summary(
                    profile.get("input_latency"),
                    latency_summary,
                    label=f"run {run_number} {segment} input latency",
                )
                for column in INPUT_LATENCY_COLUMNS:
                    input_latency_matrix[(run_number, segment, column)] = (
                        materialized_latency[column]
                    )
                normalized_profile["input_latency_schema"] = INPUT_LATENCY_SCHEMA
                normalized_profile["input_latency"] = latency_summary
            normalized_profiles.append(normalized_profile)
        normalized_run = dict(run)
        normalized_run["capture_schema"] = run.get("capture_schema", run.get("schema", 0))
        normalized_run["schema"] = SCHEMA
        normalized_run["profiles"] = normalized_profiles
        normalized_runs.append(normalized_run)
    return (
        normalized_runs,
        matrix,
        phase_matrix,
        flight_matrix,
        gpu_stage_matrix,
        input_latency_matrix,
    )


def _frame_phase_scope_summary(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
    run_numbers: Sequence[int],
    segment: str | None,
) -> dict[str, object]:
    selected = [
        (run_number, profile_segment)
        for run_number in run_numbers
        if runs[run_number - 1].get("client") == "bbb"
        for profile_segment in CAPTURE_SEGMENTS
        if segment is None or profile_segment == segment
        if (run_number, profile_segment, "application_frame_interval") in matrix
    ]
    if not selected:
        return {}
    intervals = [
        value
        for run_number, profile_segment in selected
        for value in matrix[(run_number, profile_segment, "application_frame_interval")]
    ]
    available_phases = tuple(
        phase
        for phase in FRAME_PHASE_COLUMNS
        if all(
            (run_number, profile_segment, phase) in matrix
            for run_number, profile_segment in selected
        )
    )
    summaries = {}
    for phase in available_phases:
        values = [
            value
            for run_number, profile_segment in selected
            for value in matrix[(run_number, profile_segment, phase)]
        ]
        next_phase_values = [
            value
            for run_number, profile_segment in selected
            for value in matrix[(run_number, profile_segment, phase)][:-1]
        ]
        next_application_intervals = [
            value
            for run_number, profile_segment in selected
            for value in matrix[
                (run_number, profile_segment, "application_frame_interval")
            ][1:]
        ]
        summaries[phase] = summarize_frame_phase_ms(
            values,
            intervals,
            next_phase_values=next_phase_values,
            next_application_intervals=next_application_intervals,
        )
    return summaries


def _build_frame_phase_diagnostics(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
) -> dict[str, object] | None:
    all_runs = tuple(range(1, len(runs) + 1))
    combined = _frame_phase_scope_summary(matrix, runs, all_runs, None)
    if not combined:
        return None
    scopes = [
        {"scope_type": "combined", "scope_id": "combined", "phases": combined}
    ]
    scopes.extend(
        {
            "scope_type": "scene",
            "scope_id": segment,
            "phases": _frame_phase_scope_summary(matrix, runs, all_runs, segment),
        }
        for segment in CAPTURE_SEGMENTS
    )
    for run_number, run in enumerate(runs, start=1):
        if run.get("client") != "bbb":
            continue
        scopes.append(
            {
                "scope_type": "run",
                "scope_id": f"run-{run_number}",
                "phases": _frame_phase_scope_summary(
                    matrix, runs, (run_number,), None
                ),
            }
        )
    observed_current = any(key[2] == "post_present_fence_submit" for key in matrix)
    observed_legacy = any(
        key[2] == "application_frame_interval"
        and (key[0], key[1], "post_present_fence_submit") not in matrix
        for key in matrix
    )
    schema = (
        "renderer-frame-phases-mixed-v2-v3"
        if observed_current and observed_legacy
        else FRAME_PHASE_SCHEMA
        if observed_current
        else LEGACY_FRAME_PHASE_SCHEMA
    )
    return {
        "schema": schema,
        "units": "milliseconds",
        "correlation_target": "application_frame_interval",
        "primary_correlation": "phase[n]-to-interval[n+1]",
        "phase_to_interval_lag_frames": 1,
        "scopes": scopes,
    }


def _frame_flight_scope_summary(
    flight_matrix: Mapping[tuple[int, str, str], Sequence[float]],
    phase_matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
    run_numbers: Sequence[int],
    segment: str | None,
) -> dict[str, object]:
    selected = [
        (run_number, profile_segment)
        for run_number in run_numbers
        if runs[run_number - 1].get("client") == "bbb"
        for profile_segment in CAPTURE_SEGMENTS
        if segment is None or profile_segment == segment
        if (run_number, profile_segment, FRAME_FLIGHT_COLUMNS[0]) in flight_matrix
    ]
    if not selected:
        return {}
    samples = {
        column: [
            value
            for run_number, profile_segment in selected
            for value in flight_matrix[(run_number, profile_segment, column)]
        ]
        for column in FRAME_FLIGHT_COLUMNS
    }
    intervals = [
        value
        for run_number, profile_segment in selected
        for value in phase_matrix[
            (run_number, profile_segment, "application_frame_interval")
        ]
    ]
    next_samples = {
        column: [
            value
            for run_number, profile_segment in selected
            for value in flight_matrix[(run_number, profile_segment, column)][:-1]
        ]
        for column in FRAME_FLIGHT_COLUMNS
    }
    next_intervals = [
        value
        for run_number, profile_segment in selected
        for value in phase_matrix[
            (run_number, profile_segment, "application_frame_interval")
        ][1:]
    ]
    return summarize_frame_flight(
        samples,
        intervals,
        next_samples=next_samples,
        next_application_intervals=next_intervals,
    )


def _build_frame_flight_diagnostics(
    flight_matrix: Mapping[tuple[int, str, str], Sequence[float]],
    phase_matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
) -> dict[str, object] | None:
    all_runs = tuple(range(1, len(runs) + 1))
    combined = _frame_flight_scope_summary(
        flight_matrix, phase_matrix, runs, all_runs, None
    )
    if not combined:
        return None
    scopes = [
        {"scope_type": "combined", "scope_id": "combined", "metrics": combined}
    ]
    scopes.extend(
        {
            "scope_type": "scene",
            "scope_id": segment,
            "metrics": _frame_flight_scope_summary(
                flight_matrix, phase_matrix, runs, all_runs, segment
            ),
        }
        for segment in CAPTURE_SEGMENTS
    )
    for run_number, run in enumerate(runs, start=1):
        if run.get("client") == "bbb":
            scopes.append(
                {
                    "scope_type": "run",
                    "scope_id": f"run-{run_number}",
                    "metrics": _frame_flight_scope_summary(
                        flight_matrix,
                        phase_matrix,
                        runs,
                        (run_number,),
                        None,
                    ),
                }
            )
    return {
        "schema": FRAME_FLIGHT_SCHEMA,
        "duration_units": "milliseconds",
        "count_units": "count",
        "timestamp_semantics": "wgpu-callback-observation-not-hardware-completion",
        "correlation_target": "application_frame_interval",
        "primary_correlation": "observation[n]-to-interval[n+1]",
        "phase_to_interval_lag_frames": 1,
        "scopes": scopes,
    }


def _gpu_stage_scope_summary(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
    run_numbers: Sequence[int],
    segment: str | None,
) -> dict[str, object]:
    selected = [
        (run_number, profile_segment)
        for run_number in run_numbers
        if runs[run_number - 1].get("client") == "bbb"
        for profile_segment in CAPTURE_SEGMENTS
        if segment is None or profile_segment == segment
        if (run_number, profile_segment, GPU_STAGE_COLUMNS[0]) in matrix
    ]
    if not selected:
        return {}
    samples = {
        column: [
            value
            for run_number, profile_segment in selected
            for value in matrix[(run_number, profile_segment, column)]
        ]
        for column in GPU_STAGE_COLUMNS
    }
    return summarize_gpu_stages(samples)


def _build_gpu_stage_diagnostics(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
) -> dict[str, object] | None:
    all_runs = tuple(range(1, len(runs) + 1))
    combined = _gpu_stage_scope_summary(matrix, runs, all_runs, None)
    if not combined:
        return None
    scopes = [
        {"scope_type": "combined", "scope_id": "combined", "metrics": combined}
    ]
    scopes.extend(
        {
            "scope_type": "scene",
            "scope_id": segment,
            "metrics": _gpu_stage_scope_summary(matrix, runs, all_runs, segment),
        }
        for segment in CAPTURE_SEGMENTS
    )
    for run_number in all_runs:
        if (run_number, CAPTURE_SEGMENTS[0], GPU_STAGE_COLUMNS[0]) not in matrix:
            continue
        scopes.append(
            {
                "scope_type": "run",
                "scope_id": f"run-{run_number}",
                "metrics": _gpu_stage_scope_summary(
                    matrix, runs, (run_number,), None
                ),
            }
        )
    return {
        "schema": GPU_STAGE_SCHEMA,
        "units": "milliseconds",
        "timestamp_strategy": WGPU30_GPU_TIMESTAMP_STRATEGY,
        "correlation_target": "same-submission-application-frame-interval",
        "total_duration_check": "ordered-stage-sum-within-rounding-tolerance",
        "scopes": scopes,
    }


def _input_latency_scope_summary(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
    run_numbers: Sequence[int],
    segment: str | None,
) -> dict[str, object]:
    selected = [
        (run_number, profile_segment)
        for run_number in run_numbers
        if runs[run_number - 1].get("client") == "bbb"
        for profile_segment in CAPTURE_SEGMENTS
        if segment is None or profile_segment == segment
        if (run_number, profile_segment, "sequence") in matrix
    ]
    if not selected:
        return {}
    samples = {
        column: [
            value
            for run_number, profile_segment in selected
            for value in matrix[(run_number, profile_segment, column)]
        ]
        for column in INPUT_LATENCY_COLUMNS
    }
    return {
        "sample_count": len(samples["sequence"]),
        "minimum_sequence": int(min(samples["sequence"])),
        "maximum_sequence": int(max(samples["sequence"])),
        "superseded_events": int(sum(samples["superseded_events"])),
        "out_of_order_events": int(sum(samples["out_of_order_events"])),
        "terminal_boundary": "present-call-return-software-boundary-not-display-or-photon",
        "stages": {
            column: summarize_durations_ms(samples[column])
            for column in INPUT_LATENCY_DURATION_COLUMNS
        },
    }


def _build_input_latency_diagnostics(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
) -> dict[str, object] | None:
    all_runs = tuple(range(1, len(runs) + 1))
    combined = _input_latency_scope_summary(matrix, runs, all_runs, None)
    if not combined:
        return None
    scopes = [
        {"scope_type": "combined", "scope_id": "combined", "summary": combined}
    ]
    for segment in CAPTURE_SEGMENTS:
        summary = _input_latency_scope_summary(matrix, runs, all_runs, segment)
        if summary:
            scopes.append(
                {"scope_type": "scene", "scope_id": segment, "summary": summary}
            )
    for run_number, run in enumerate(runs, start=1):
        if run.get("client") != "bbb":
            continue
        summary = _input_latency_scope_summary(matrix, runs, (run_number,), None)
        if summary:
            scopes.append(
                {"scope_type": "run", "scope_id": str(run_number), "summary": summary}
            )
    return {
        "schema": INPUT_LATENCY_SCHEMA,
        "terminal_boundary": "present-call-return-software-boundary-not-display-or-photon",
        "scopes": scopes,
    }


def _scoped_summary(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
    run_numbers: Sequence[int],
    client: str,
    metric: str,
    segment: str | None = None,
) -> dict[str, object]:
    values: list[float] = []
    segments = (segment,) if segment is not None else CAPTURE_SEGMENTS
    for run_number in run_numbers:
        if runs[run_number - 1].get("client") != client:
            continue
        for capture_segment in segments:
            values.extend(matrix[(run_number, capture_segment, metric)])
    return summarize_durations_ms(values)


def _metric_stability_comparison(
    bbb: Mapping[str, object], official: Mapping[str, object], metric: str
) -> dict[str, object]:
    lower_is_better = (
        "mean_ms",
        "p50_ms",
        "p95_ms",
        "p99_ms",
        "stddev_ms",
        "p99_minus_p50_ms",
    )
    checks: list[dict[str, object]] = []
    for field in lower_is_better:
        bbb_value = float(bbb[field])
        official_value = float(official[field])
        checks.append(
            {
                "field": field,
                "direction": "lower-or-equal",
                "bbb": bbb_value,
                "official": official_value,
                "passed": bbb_value <= official_value,
            }
        )
    if metric == "application_frame_time":
        for field in ("mean_fps", "one_percent_low_fps"):
            bbb_value = float(bbb[field])
            official_value = float(official[field])
            checks.append(
                {
                    "field": field,
                    "direction": "higher-or-equal",
                    "bbb": bbb_value,
                    "official": official_value,
                    "passed": bbb_value >= official_value,
                }
            )
    bbb_hitches = bbb["hitch_counts"]
    official_hitches = official["hitch_counts"]
    assert isinstance(bbb_hitches, Mapping) and isinstance(official_hitches, Mapping)
    for threshold in (f">={value:.3f}ms" for value in HITCH_THRESHOLDS_MS):
        bbb_value = int(bbb_hitches[threshold])
        official_value = int(official_hitches[threshold])
        checks.append(
            {
                "field": f"hitch_counts.{threshold}",
                "direction": "lower-or-equal",
                "bbb": bbb_value,
                "official": official_value,
                "passed": bbb_value <= official_value,
            }
        )
    failed = [str(check["field"]) for check in checks if not check["passed"]]
    bbb_cv = float(bbb["coefficient_of_variation"])
    official_cv = float(official["coefficient_of_variation"])
    cv_worse = bbb_cv > official_cv
    if not cv_worse:
        cv_classification = "not-worse"
    elif failed:
        cv_classification = "diagnostic-coincident-with-absolute-regression"
    else:
        cv_classification = "diagnostic-only-absolute-metrics-pass"
    return {
        "status": "passed" if not failed else "failed",
        "bbb": dict(bbb),
        "official": dict(official),
        "checks": checks,
        "failed_absolute_metrics": failed,
        "coefficient_of_variation": {
            "bbb": bbb_cv,
            "official": official_cv,
            "bbb_is_worse": cv_worse,
            "classification": cv_classification,
            "blocking_by_itself": False,
        },
    }


def _stability_scope(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
    *,
    scope_type: str,
    scope_id: str,
    run_numbers: Sequence[int],
    segment: str | None = None,
) -> dict[str, object]:
    metrics = {
        metric: _metric_stability_comparison(
            _scoped_summary(matrix, runs, run_numbers, "bbb", metric, segment),
            _scoped_summary(matrix, runs, run_numbers, "vanilla", metric, segment),
            metric,
        )
        for metric in ("application_frame_time", "gpu_duration")
    }
    return {
        "scope_type": scope_type,
        "scope_id": scope_id,
        "runs": list(run_numbers),
        "segment": segment,
        "status": (
            "passed"
            if all(metric["status"] == "passed" for metric in metrics.values())
            else "failed"
        ),
        "metrics": metrics,
    }


def _build_stability_acceptance(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    runs: Sequence[Mapping[str, object]],
) -> dict[str, object]:
    all_runs = tuple(range(1, len(runs) + 1))
    scopes = [
        _stability_scope(
            matrix,
            runs,
            scope_type="combined",
            scope_id="combined",
            run_numbers=all_runs,
        )
    ]
    scopes.extend(
        _stability_scope(
            matrix,
            runs,
            scope_type="scene",
            scope_id=segment,
            run_numbers=all_runs,
            segment=segment,
        )
        for segment in CAPTURE_SEGMENTS
    )
    scopes.extend(
        _stability_scope(
            matrix,
            runs,
            scope_type="abba_round",
            scope_id=f"round-{round_index}",
            run_numbers=round_runs,
        )
        for round_index, round_runs in enumerate(FORMAL_ROUNDS, start=1)
    )
    for pair_index, pair in enumerate(FORMAL_RUN_PAIRS, start=1):
        scopes.append(
            _stability_scope(
                matrix,
                runs,
                scope_type="run_pair",
                scope_id=f"pair-{pair_index}-all-scenes",
                run_numbers=pair,
            )
        )
        scopes.extend(
            _stability_scope(
                matrix,
                runs,
                scope_type="run_pair",
                scope_id=f"pair-{pair_index}-{segment}",
                run_numbers=pair,
                segment=segment,
            )
            for segment in CAPTURE_SEGMENTS
        )
    failures: list[dict[str, object]] = []
    cv_diagnostics: list[dict[str, object]] = []
    for scope in scopes:
        metrics = scope["metrics"]
        assert isinstance(metrics, Mapping)
        for metric, comparison in metrics.items():
            assert isinstance(comparison, Mapping)
            cv = comparison["coefficient_of_variation"]
            assert isinstance(cv, Mapping)
            if cv["bbb_is_worse"]:
                cv_diagnostics.append(
                    {
                        "scope_type": scope["scope_type"],
                        "scope_id": scope["scope_id"],
                        "metric": metric,
                        **dict(cv),
                    }
                )
            failed = comparison["failed_absolute_metrics"]
            if failed:
                failures.append(
                    {
                        "code": "absolute_stability_regression",
                        "scope_type": scope["scope_type"],
                        "scope_id": scope["scope_id"],
                        "runs": scope["runs"],
                        "segment": scope["segment"],
                        "metric": metric,
                        "failed_metrics": list(failed),
                        "cv_classification": cv["classification"],
                    }
                )
    return {
        "policy": STABILITY_POLICY,
        "status": "passed" if not failures else "failed",
        "cv_role": "diagnostic-only-when-absolute-metrics-pass",
        "required_scopes": ["combined", "scene", "abba_round", "run_pair"],
        "scopes": scopes,
        "cv_diagnostics": cv_diagnostics,
        "failures": failures,
    }


def evaluate_candidate_fps_noninferiority(
    baseline_mean_fps: float, candidate_mean_fps: float, margin: float
) -> dict[str, object]:
    if not all(math.isfinite(value) and value > 0.0 for value in (baseline_mean_fps, candidate_mean_fps)):
        raise RetinaAcceptanceError("baseline/candidate FPS must be finite and positive")
    if not math.isfinite(margin) or not 0.0 <= margin <= MAX_BASELINE_FPS_MARGIN:
        raise RetinaAcceptanceError(
            f"candidate FPS margin must be in 0..={MAX_BASELINE_FPS_MARGIN:.3f}"
        )
    minimum = baseline_mean_fps * (1.0 - margin)
    return {
        "baseline_mean_fps": baseline_mean_fps,
        "candidate_mean_fps": candidate_mean_fps,
        "relative_margin": margin,
        "minimum_candidate_mean_fps": minimum,
        "passed": candidate_mean_fps >= minimum,
    }


def _candidate_required_scopes() -> set[tuple[str, str]]:
    required = {("combined", "combined")}
    required.update(("scene", segment) for segment in CAPTURE_SEGMENTS)
    required.update(
        ("abba_round", f"round-{round_index}")
        for round_index in range(1, len(FORMAL_ROUNDS) + 1)
    )
    for pair_index in range(1, len(FORMAL_RUN_PAIRS) + 1):
        required.add(("run_pair", f"pair-{pair_index}-all-scenes"))
        required.update(
            ("run_pair", f"pair-{pair_index}-{segment}")
            for segment in CAPTURE_SEGMENTS
        )
    return required


def _candidate_parity_failures(evidence: Mapping[str, object]) -> list[dict[str, object]]:
    parity = evidence.get("parity_contract")
    if not isinstance(parity, Mapping):
        return [{"code": "bbb_candidate_parity_contract_missing"}]
    expected: tuple[tuple[str, object], ...] = (
        ("status", "passed"),
        ("drawable_pixels", [2560, 1600]),
        ("window_mode", "fullscreen"),
        ("vsync", False),
        ("framerate_limit", "unlimited"),
        ("scene_match", True),
        ("workload_hashes_match", True),
        ("draw_workload_match", True),
        ("input_match", True),
    )
    mismatches = [
        {"field": field, "expected": expected_value, "actual": parity.get(field)}
        for field, expected_value in expected
        if parity.get(field) != expected_value
    ]
    return (
        [{"code": "bbb_candidate_parity_contract_mismatch", "mismatches": mismatches}]
        if mismatches
        else []
    )


def _candidate_scheduling_failures(evidence: Mapping[str, object]) -> list[dict[str, object]]:
    """Validate the v2 low-latency surface candidate without invalidating v1 history."""

    if evidence.get("schema") == LEGACY_CANDIDATE_AB_SCHEMA:
        return []
    if evidence.get("schema") == WGPU30_OPTIMIZATION_AB_SCHEMA:
        return _wgpu30_optimization_receipt_and_scheduling_failures(evidence)
    if evidence.get("schema") == WGPU30_CANDIDATE_AB_SCHEMA:
        return _wgpu30_candidate_receipt_and_scheduling_failures(evidence)
    contract = evidence.get("scheduling_contract")
    if not isinstance(contract, Mapping):
        return [{"code": "bbb_candidate_scheduling_contract_missing"}]
    expected: tuple[tuple[str, object], ...] = (
        ("status", "passed"),
        ("platform", "macos"),
        ("present_mode", "Immediate"),
        ("baseline_desired_maximum_frame_latency", 2),
        ("candidate_desired_maximum_frame_latency", 1),
        ("baseline_expected_metal_drawable_count", 3),
        ("candidate_expected_metal_drawable_count", 2),
        ("baseline_in_flight_submission_budget", 1),
        ("candidate_in_flight_submission_budget", 1),
        ("queue_depth_increased", False),
        ("wait_acquire_submit_present_order_unchanged", True),
        ("input_latency_policy", "wgpu-documented-low-latency-surface-hint"),
    )
    mismatches = [
        {"field": field, "expected": expected_value, "actual": contract.get(field)}
        for field, expected_value in expected
        if contract.get(field) != expected_value
    ]
    failures = (
        [{"code": "bbb_candidate_scheduling_contract_mismatch", "mismatches": mismatches}]
        if mismatches
        else []
    )
    runs = evidence.get("runs")
    if not isinstance(runs, list) or len(runs) != len(CANDIDATE_AB_ORDER):
        failures.append(
            {
                "code": "bbb_candidate_surface_runs_missing",
                "expected_count": len(CANDIDATE_AB_ORDER),
                "actual_count": len(runs) if isinstance(runs, list) else None,
            }
        )
        return failures
    for sequence, (run, expected_variant) in enumerate(
        zip(runs, CANDIDATE_AB_ORDER, strict=True), start=1
    ):
        if not isinstance(run, Mapping):
            failures.append(
                {"code": "bbb_candidate_surface_run_malformed", "run": sequence}
            )
            continue
        expected_latency = 2 if expected_variant == "baseline" else 1
        expected_drawables = expected_latency + 1
        presentation = run.get("surface_presentation")
        run_mismatches: list[dict[str, object]] = []
        expected_run: tuple[tuple[str, object], ...] = (
            ("variant", expected_variant),
            ("status", "passed"),
        )
        run_mismatches.extend(
            {"field": field, "expected": value, "actual": run.get(field)}
            for field, value in expected_run
            if run.get(field) != value
        )
        if not isinstance(presentation, Mapping):
            run_mismatches.append(
                {"field": "surface_presentation", "expected": "mapping", "actual": presentation}
            )
        else:
            expected_presentation: tuple[tuple[str, object], ...] = (
                ("api", "metal"),
                ("present_mode", "Immediate"),
                ("desired_maximum_frame_latency", expected_latency),
                ("expected_metal_drawable_count", expected_drawables),
            )
            run_mismatches.extend(
                {
                    "field": f"surface_presentation.{field}",
                    "expected": value,
                    "actual": presentation.get(field),
                }
                for field, value in expected_presentation
                if presentation.get(field) != value
            )
        if run_mismatches:
            failures.append(
                {
                    "code": "bbb_candidate_surface_run_mismatch",
                    "run": sequence,
                    "mismatches": run_mismatches,
                }
            )
    return failures


def _wgpu30_optimization_receipt_and_scheduling_failures(
    evidence: Mapping[str, object],
) -> list[dict[str, object]]:
    """Require registry wgpu 30 and one timing scope on both A/B sides."""

    failures: list[dict[str, object]] = []
    try:
        committed_receipt = load_committed_wgpu_dependency_receipt()
    except RetinaAcceptanceError as error:
        return [{"code": "bbb_candidate_dependency_receipt_invalid", "error": str(error)}]
    receipt_sha256 = file_digest(WGPU_DEPENDENCY_RECEIPT_PATH, "sha256")
    for variant in ("baseline", "candidate"):
        if evidence.get(f"{variant}_dependency_receipt") != committed_receipt:
            failures.append(
                {"code": f"bbb_{variant}_dependency_receipt_mismatch"}
            )
        if evidence.get(f"{variant}_dependency_receipt_sha256") != receipt_sha256:
            failures.append(
                {"code": f"bbb_{variant}_dependency_receipt_hash_mismatch"}
            )

    contract = evidence.get("scheduling_contract")
    if not isinstance(contract, Mapping):
        failures.append({"code": "bbb_candidate_scheduling_contract_missing"})
        return failures
    expected_contract: tuple[tuple[str, object], ...] = (
        ("status", "passed"),
        ("platform", "macos"),
        ("present_mode", "Immediate"),
        ("baseline_desired_maximum_frame_latency", 2),
        ("candidate_desired_maximum_frame_latency", 2),
        ("baseline_expected_metal_drawable_count", 3),
        ("candidate_expected_metal_drawable_count", 3),
        ("baseline_actual_metal_drawable_count", 3),
        ("candidate_actual_metal_drawable_count", 3),
        ("baseline_in_flight_submission_budget", 1),
        ("candidate_in_flight_submission_budget", 1),
        ("queue_depth_increased", False),
        ("wait_acquire_submit_present_order_unchanged", True),
        ("input_latency_policy", "unchanged-single-in-flight"),
    )
    mismatches = [
        {"field": field, "expected": expected, "actual": contract.get(field)}
        for field, expected in expected_contract
        if contract.get(field) != expected
    ]
    if mismatches:
        failures.append(
            {"code": "bbb_candidate_scheduling_contract_mismatch", "mismatches": mismatches}
        )
    fence_strategies: dict[str, str] | None = None
    if (
        "baseline_submission_fence_strategy" in contract
        or "candidate_submission_fence_strategy" in contract
    ):
        fence_strategies = {}
        for variant in ("baseline", "candidate"):
            field = f"{variant}_submission_fence_strategy"
            expected = contract.get(field)
            if not isinstance(expected, str) or not expected:
                failures.append(
                    {
                        "code": "bbb_candidate_scheduling_contract_mismatch",
                        "mismatches": [
                            {
                                "field": field,
                                "expected": "non-empty predeclared strategy",
                                "actual": expected,
                            }
                        ],
                    }
                )
            else:
                fence_strategies[variant] = expected

    expected_runtime = {
        **WGPU_COMPONENT_VERSIONS,
        "upstream_metal_fence_commit": WGPU_METAL_FENCE_COMMIT,
        "upstream_metal_fence_strategy": "completion-handler-condition-variable",
    }
    if evidence.get("gpu_timestamp_strategy") != WGPU30_GPU_TIMESTAMP_STRATEGY:
        failures.append({"code": "bbb_candidate_gpu_timestamp_strategy_mismatch"})
    candidate_binary_sha256 = evidence.get("candidate_bbb_binary_sha256")
    if (
        not isinstance(candidate_binary_sha256, str)
        or re.fullmatch(r"[0-9a-f]{64}", candidate_binary_sha256) is None
    ):
        failures.append({"code": "bbb_candidate_binary_hash_missing"})

    runs = evidence.get("runs")
    if not isinstance(runs, list) or len(runs) != len(CANDIDATE_AB_ORDER):
        failures.append(
            {
                "code": "bbb_candidate_surface_runs_missing",
                "expected_count": len(CANDIDATE_AB_ORDER),
                "actual_count": len(runs) if isinstance(runs, list) else None,
            }
        )
        return failures
    for sequence, (run, expected_variant) in enumerate(
        zip(runs, CANDIDATE_AB_ORDER, strict=True), start=1
    ):
        run_mismatches: list[dict[str, object]] = []
        if not isinstance(run, Mapping):
            failures.append({"code": "bbb_candidate_surface_run_malformed", "run": sequence})
            continue
        for field, expected in (
            ("sequence", sequence),
            ("variant", expected_variant),
            ("status", "passed"),
        ):
            if run.get(field) != expected:
                run_mismatches.append(
                    {"field": field, "expected": expected, "actual": run.get(field)}
                )
        presentation = run.get("surface_presentation")
        expected_presentation: tuple[tuple[str, object], ...] = (
            ("api", "metal"),
            ("present_mode", "Immediate"),
            ("desired_maximum_frame_latency", 2),
            ("expected_metal_drawable_count", 3),
            ("actual_metal_drawable_count", 3),
            ("actual_drawable_pixels", [2560, 1600]),
            ("in_flight_submission_budget", 1),
        )
        if not isinstance(presentation, Mapping):
            run_mismatches.append(
                {"field": "surface_presentation", "expected": "mapping", "actual": presentation}
            )
        else:
            run_mismatches.extend(
                {
                    "field": f"surface_presentation.{field}",
                    "expected": expected,
                    "actual": presentation.get(field),
                }
                for field, expected in expected_presentation
                if presentation.get(field) != expected
            )
            if fence_strategies is not None and expected_variant in fence_strategies:
                expected_fence_strategy = fence_strategies[expected_variant]
                if presentation.get("submission_fence_strategy") != expected_fence_strategy:
                    run_mismatches.append(
                        {
                            "field": "surface_presentation.submission_fence_strategy",
                            "expected": expected_fence_strategy,
                            "actual": presentation.get("submission_fence_strategy"),
                        }
                    )
        expected_binary_sha256 = (
            WGPU30_OPTIMIZATION_BASELINE_BINARY_SHA256
            if expected_variant == "baseline"
            else candidate_binary_sha256
        )
        for field, expected in (
            ("binary_sha256", expected_binary_sha256),
            ("runtime_dependency_receipt", expected_runtime),
            ("gpu_timestamp_strategy", WGPU30_GPU_TIMESTAMP_STRATEGY),
        ):
            if run.get(field) != expected:
                run_mismatches.append(
                    {"field": field, "expected": expected, "actual": run.get(field)}
                )
        if run_mismatches:
            failures.append(
                {
                    "code": "bbb_candidate_surface_run_mismatch",
                    "run": sequence,
                    "mismatches": run_mismatches,
                }
            )
    return failures


def _wgpu30_candidate_receipt_and_scheduling_failures(
    evidence: Mapping[str, object],
) -> list[dict[str, object]]:
    failures: list[dict[str, object]] = []
    try:
        committed_receipt = load_committed_wgpu_dependency_receipt()
    except RetinaAcceptanceError as error:
        return [{"code": "bbb_candidate_dependency_receipt_invalid", "error": str(error)}]
    if evidence.get("candidate_dependency_receipt") != committed_receipt:
        failures.append({"code": "bbb_candidate_dependency_receipt_mismatch"})
    expected_receipt_sha256 = file_digest(WGPU_DEPENDENCY_RECEIPT_PATH, "sha256")
    if evidence.get("candidate_dependency_receipt_sha256") != expected_receipt_sha256:
        failures.append({"code": "bbb_candidate_dependency_receipt_hash_mismatch"})

    contract = evidence.get("scheduling_contract")
    if not isinstance(contract, Mapping):
        failures.append({"code": "bbb_candidate_scheduling_contract_missing"})
        return failures
    expected_contract: tuple[tuple[str, object], ...] = (
        ("status", "passed"),
        ("platform", "macos"),
        ("present_mode", "Immediate"),
        ("baseline_desired_maximum_frame_latency", 2),
        ("candidate_desired_maximum_frame_latency", 2),
        ("baseline_expected_metal_drawable_count", 3),
        ("candidate_expected_metal_drawable_count", 3),
        ("baseline_actual_metal_drawable_count", 3),
        ("candidate_actual_metal_drawable_count", 3),
        ("baseline_in_flight_submission_budget", 1),
        ("candidate_in_flight_submission_budget", 1),
        ("queue_depth_increased", False),
        ("wait_acquire_submit_present_order_unchanged", True),
        ("input_latency_policy", "unchanged-single-in-flight"),
    )
    mismatches = [
        {"field": field, "expected": expected, "actual": contract.get(field)}
        for field, expected in expected_contract
        if contract.get(field) != expected
    ]
    if mismatches:
        failures.append(
            {"code": "bbb_candidate_scheduling_contract_mismatch", "mismatches": mismatches}
        )

    runs = evidence.get("runs")
    if not isinstance(runs, list) or len(runs) != len(CANDIDATE_AB_ORDER):
        failures.append(
            {
                "code": "bbb_candidate_surface_runs_missing",
                "expected_count": len(CANDIDATE_AB_ORDER),
                "actual_count": len(runs) if isinstance(runs, list) else None,
            }
        )
        return failures
    expected_candidate_versions = WGPU_COMPONENT_VERSIONS
    rebuilt_baseline_sha256 = evidence.get("actual_rebuilt_baseline_binary_sha256")
    if (
        not isinstance(rebuilt_baseline_sha256, str)
        or re.fullmatch(r"[0-9a-f]{64}", rebuilt_baseline_sha256) is None
    ):
        failures.append({"code": "bbb_candidate_rebuilt_baseline_binary_hash_missing"})
    for sequence, (run, expected_variant) in enumerate(
        zip(runs, CANDIDATE_AB_ORDER, strict=True), start=1
    ):
        run_mismatches: list[dict[str, object]] = []
        if not isinstance(run, Mapping):
            failures.append({"code": "bbb_candidate_surface_run_malformed", "run": sequence})
            continue
        for field, expected in (("sequence", sequence), ("variant", expected_variant), ("status", "passed")):
            if run.get(field) != expected:
                run_mismatches.append(
                    {"field": field, "expected": expected, "actual": run.get(field)}
                )
        presentation = run.get("surface_presentation")
        if not isinstance(presentation, Mapping):
            run_mismatches.append(
                {"field": "surface_presentation", "expected": "mapping", "actual": presentation}
            )
        else:
            expected_presentation: tuple[tuple[str, object], ...] = (
                ("api", "metal"),
                ("present_mode", "Immediate"),
                ("desired_maximum_frame_latency", 2),
                ("expected_metal_drawable_count", 3),
                ("actual_metal_drawable_count", 3),
                ("actual_drawable_pixels", [2560, 1600]),
                ("in_flight_submission_budget", 1),
            )
            run_mismatches.extend(
                {
                    "field": f"surface_presentation.{field}",
                    "expected": expected,
                    "actual": presentation.get(field),
                }
                for field, expected in expected_presentation
                if presentation.get(field) != expected
            )
        binary_sha256 = run.get("binary_sha256")
        expected_binary_sha256 = (
            rebuilt_baseline_sha256
            if expected_variant == "baseline"
            else evidence.get("candidate_bbb_binary_sha256")
        )
        if binary_sha256 != expected_binary_sha256:
            run_mismatches.append(
                {
                    "field": "binary_sha256",
                    "expected": expected_binary_sha256,
                    "actual": binary_sha256,
                }
            )
        if expected_variant == "candidate":
            runtime = run.get("runtime_dependency_receipt")
            expected_runtime = {
                **expected_candidate_versions,
                "upstream_metal_fence_commit": WGPU_METAL_FENCE_COMMIT,
                "upstream_metal_fence_strategy": "completion-handler-condition-variable",
            }
            if runtime != expected_runtime:
                run_mismatches.append(
                    {
                        "field": "runtime_dependency_receipt",
                        "expected": expected_runtime,
                        "actual": runtime,
                    }
                )
        if run_mismatches:
            failures.append(
                {
                    "code": "bbb_candidate_surface_run_mismatch",
                    "run": sequence,
                    "mismatches": run_mismatches,
                }
            )
    return failures


def _candidate_metric_stability_comparison(
    baseline: Mapping[str, object],
    candidate: Mapping[str, object],
    metric: str,
    fps_margin: float,
) -> dict[str, object]:
    lower_is_better = ["p50_ms", "p95_ms", "p99_ms", "stddev_ms", "p99_minus_p50_ms"]
    if metric == "gpu_duration":
        lower_is_better.insert(0, "mean_ms")
    checks: list[dict[str, object]] = []
    for field in lower_is_better:
        baseline_value = float(baseline[field])
        candidate_value = float(candidate[field])
        if not all(math.isfinite(value) and value >= 0.0 for value in (baseline_value, candidate_value)):
            raise RetinaAcceptanceError(f"candidate {metric} {field} must be finite and non-negative")
        checks.append(
            {
                "field": field,
                "direction": "lower-or-equal",
                "baseline": baseline_value,
                "candidate": candidate_value,
                "passed": candidate_value <= baseline_value,
            }
        )
    if metric == "application_frame_time":
        baseline_mean = float(baseline["mean_ms"])
        candidate_mean = float(candidate["mean_ms"])
        maximum_candidate_mean = baseline_mean / (1.0 - fps_margin)
        checks.append(
            {
                "field": "mean_ms",
                "direction": "lower-with-predeclared-fps-margin",
                "baseline": baseline_mean,
                "candidate": candidate_mean,
                "maximum_candidate": maximum_candidate_mean,
                "passed": candidate_mean <= maximum_candidate_mean,
            }
        )
        baseline_low = float(baseline["one_percent_low_fps"])
        candidate_low = float(candidate["one_percent_low_fps"])
        checks.append(
            {
                "field": "one_percent_low_fps",
                "direction": "higher-or-equal",
                "baseline": baseline_low,
                "candidate": candidate_low,
                "passed": candidate_low >= baseline_low,
            }
        )
    baseline_hitches = baseline["hitch_counts"]
    candidate_hitches = candidate["hitch_counts"]
    if not isinstance(baseline_hitches, Mapping) or not isinstance(candidate_hitches, Mapping):
        raise RetinaAcceptanceError(f"candidate {metric} hitch counts are malformed")
    for threshold in (f">={value:.3f}ms" for value in HITCH_THRESHOLDS_MS):
        baseline_value = int(baseline_hitches[threshold])
        candidate_value = int(candidate_hitches[threshold])
        checks.append(
            {
                "field": f"hitch_counts.{threshold}",
                "direction": "lower-or-equal",
                "baseline": baseline_value,
                "candidate": candidate_value,
                "passed": candidate_value <= baseline_value,
            }
        )
    failed = [str(check["field"]) for check in checks if not check["passed"]]
    baseline_cv = float(baseline["coefficient_of_variation"])
    candidate_cv = float(candidate["coefficient_of_variation"])
    cv_worse = candidate_cv > baseline_cv
    if not cv_worse:
        cv_classification = "not-worse"
    elif failed:
        cv_classification = "diagnostic-coincident-with-absolute-regression"
    else:
        cv_classification = "diagnostic-only-absolute-metrics-pass"
    improved = any(
        (
            check["candidate"] < check["baseline"]
            if check["direction"].startswith("lower")
            else check["candidate"] > check["baseline"]
        )
        for check in checks
        if check["field"] != "mean_ms" or metric == "gpu_duration"
    )
    return {
        "status": "passed" if not failed else "failed",
        "baseline": dict(baseline),
        "candidate": dict(candidate),
        "checks": checks,
        "failed_absolute_metrics": failed,
        "has_absolute_improvement": improved,
        "coefficient_of_variation": {
            "baseline": baseline_cv,
            "candidate": candidate_cv,
            "candidate_is_worse": cv_worse,
            "classification": cv_classification,
            "blocking_by_itself": False,
        },
    }


def validate_candidate_stability_evidence(
    evidence: Mapping[str, object], fps_margin: float
) -> dict[str, object]:
    acceptance = evidence.get("absolute_stability")
    if not isinstance(acceptance, Mapping):
        return {
            "status": "failed",
            "results": [],
            "failures": [{"code": "bbb_candidate_absolute_stability_missing"}],
        }
    scopes = acceptance.get("scopes")
    if not isinstance(scopes, list):
        return {
            "status": "failed",
            "results": [],
            "failures": [{"code": "bbb_candidate_absolute_scopes_missing"}],
        }
    required = _candidate_required_scopes()
    observed: set[tuple[str, str]] = set()
    results: list[dict[str, object]] = []
    failures: list[dict[str, object]] = []
    if acceptance.get("policy") != CANDIDATE_STABILITY_POLICY:
        failures.append({"code": "bbb_candidate_absolute_policy_mismatch"})
    combined_improved = False
    for scope in scopes:
        if not isinstance(scope, Mapping):
            failures.append({"code": "bbb_candidate_absolute_scope_malformed"})
            continue
        scope_key = (str(scope.get("scope_type")), str(scope.get("scope_id")))
        if scope_key in observed:
            failures.append(
                {"code": "bbb_candidate_absolute_scope_duplicate", "scope": list(scope_key)}
            )
        observed.add(scope_key)
        metrics = scope.get("metrics")
        if not isinstance(metrics, Mapping):
            failures.append(
                {"code": "bbb_candidate_absolute_scope_malformed", "scope": list(scope_key)}
            )
            continue
        comparisons: dict[str, object] = {}
        for metric in ("application_frame_time", "gpu_duration"):
            values = metrics.get(metric)
            try:
                if not isinstance(values, Mapping):
                    raise RetinaAcceptanceError(f"missing {metric}")
                baseline = values["baseline"]
                candidate = values["candidate"]
                if not isinstance(baseline, Mapping) or not isinstance(candidate, Mapping):
                    raise RetinaAcceptanceError(f"malformed {metric} summaries")
                comparison = _candidate_metric_stability_comparison(
                    baseline, candidate, metric, fps_margin
                )
            except (KeyError, TypeError, ValueError, RetinaAcceptanceError) as error:
                failures.append(
                    {
                        "code": "bbb_candidate_absolute_scope_malformed",
                        "scope": list(scope_key),
                        "metric": metric,
                        "error": str(error),
                    }
                )
                continue
            comparisons[metric] = comparison
            if scope_key == ("combined", "combined") and comparison["has_absolute_improvement"]:
                combined_improved = True
            failed = comparison["failed_absolute_metrics"]
            if failed:
                failures.append(
                    {
                        "code": "bbb_candidate_absolute_stability_regression",
                        "scope": list(scope_key),
                        "metric": metric,
                        "failed_metrics": list(failed),
                    }
                )
        results.append(
            {
                "scope_type": scope_key[0],
                "scope_id": scope_key[1],
                "metrics": comparisons,
            }
        )
    missing = sorted(required - observed)
    if missing:
        failures.append(
            {
                "code": "bbb_candidate_absolute_required_scopes_missing",
                "scopes": [list(item) for item in missing],
            }
        )
    unexpected = sorted(observed - required)
    if unexpected:
        failures.append(
            {
                "code": "bbb_candidate_absolute_scopes_unexpected",
                "scopes": [list(item) for item in unexpected],
            }
        )
    if not combined_improved:
        failures.append({"code": "bbb_candidate_absolute_improvement_missing"})
    return {
        "status": "passed" if not failures else "failed",
        "required_scopes": [list(item) for item in sorted(required)],
        "results": results,
        "failures": failures,
    }


def validate_candidate_fps_evidence(
    evidence: Mapping[str, object] | None, candidate_commit: str
) -> dict[str, object]:
    if evidence is None:
        return {
            "status": "failed",
            "failures": [{"code": "bbb_baseline_comparison_missing"}],
        }
    parity_failures = _candidate_parity_failures(evidence)
    if parity_failures:
        return {
            "status": "failed",
            "candidate_bbb_commit": candidate_commit,
            "maximum_margin": MAX_BASELINE_FPS_MARGIN,
            "results": [],
            "stability": None,
            "failures": parity_failures,
        }
    is_wgpu30_optimization = evidence.get("schema") == WGPU30_OPTIMIZATION_AB_SCHEMA
    expected_baseline_commit = (
        WGPU30_OPTIMIZATION_BASELINE_COMMIT
        if is_wgpu30_optimization
        else BASELINE_BBB_COMMIT
    )
    expected_baseline_report_sha256 = (
        WGPU30_OPTIMIZATION_BASELINE_REPORT_SHA256
        if is_wgpu30_optimization
        else BASELINE_BBB_REPORT_SHA256
    )
    expected_baseline_binary_sha256 = (
        WGPU30_OPTIMIZATION_BASELINE_BINARY_SHA256
        if is_wgpu30_optimization
        else BASELINE_BBB_BINARY_SHA256
    )
    failures: list[dict[str, object]] = []
    if evidence.get("schema") not in ACCEPTED_CANDIDATE_AB_SCHEMAS:
        failures.append({"code": "bbb_candidate_schema_mismatch"})
    else:
        failures.extend(_candidate_scheduling_failures(evidence))
    if evidence.get("baseline_bbb_commit") != expected_baseline_commit:
        failures.append({"code": "bbb_baseline_commit_mismatch"})
    if evidence.get("baseline_bbb_report_sha256") != expected_baseline_report_sha256:
        failures.append({"code": "bbb_baseline_report_mismatch"})
    if evidence.get("baseline_bbb_binary_sha256") != expected_baseline_binary_sha256:
        failures.append({"code": "bbb_baseline_binary_mismatch"})
    if evidence.get("candidate_bbb_commit") != candidate_commit:
        failures.append({"code": "bbb_candidate_commit_mismatch"})
    if is_wgpu30_optimization and failures:
        return {
            "status": "failed",
            "baseline_bbb_commit": expected_baseline_commit,
            "baseline_bbb_report_sha256": expected_baseline_report_sha256,
            "baseline_bbb_binary_sha256": expected_baseline_binary_sha256,
            "candidate_bbb_commit": candidate_commit,
            "maximum_margin": MAX_BASELINE_FPS_MARGIN,
            "results": [],
            "stability": None,
            "failures": failures,
        }
    if evidence.get("pairing") != "interleaved-baseline-candidate":
        failures.append({"code": "bbb_candidate_pairing_mismatch"})
    if evidence.get("margin_declared_before_capture") is not True:
        failures.append({"code": "bbb_candidate_margin_not_predeclared"})
    raw_sha256 = evidence.get("baseline_repeat_raw_sha256")
    if not isinstance(raw_sha256, str) or re.fullmatch(r"[0-9a-f]{64}", raw_sha256) is None:
        failures.append({"code": "bbb_baseline_noise_raw_hash_missing"})
    elif (
        is_wgpu30_optimization
        and raw_sha256 != WGPU30_OPTIMIZATION_BASELINE_RAW_SHA256
    ):
        failures.append({"code": "bbb_baseline_noise_raw_hash_mismatch"})
    try:
        margin = float(evidence.get("baseline_repeat_noise_margin"))
    except (TypeError, ValueError):
        margin = math.nan
    margin_is_valid = math.isfinite(margin) and 0.0 <= margin <= MAX_BASELINE_FPS_MARGIN
    if not margin_is_valid:
        failures.append({"code": "bbb_candidate_margin_out_of_range"})
    comparison_margin = margin if margin_is_valid else 0.0
    repeat_values = evidence.get("baseline_repeat_mean_fps")
    if not isinstance(repeat_values, list) or len(repeat_values) < 4:
        failures.append({"code": "bbb_baseline_noise_repeats_missing"})
    else:
        try:
            repeats = [float(value) for value in repeat_values]
            if is_wgpu30_optimization and repeats != list(
                WGPU30_OPTIMIZATION_BASELINE_REPEAT_FPS
            ):
                failures.append({"code": "bbb_baseline_noise_repeats_mismatch"})
            if not all(math.isfinite(value) and value > 0.0 for value in repeats):
                raise ValueError("non-positive baseline repeat")
            measured_noise = (max(repeats) - min(repeats)) / statistics.fmean(repeats)
            derived_margin = measured_noise
            if measured_noise > MAX_BASELINE_FPS_MARGIN:
                failures.append(
                    {
                        "code": "bbb_baseline_noise_exceeds_maximum",
                        "measured": measured_noise,
                        "maximum": MAX_BASELINE_FPS_MARGIN,
                    }
                )
            if not math.isclose(margin, derived_margin, rel_tol=1e-12, abs_tol=1e-12):
                failures.append(
                    {
                        "code": "bbb_candidate_margin_not_derived_from_baseline_noise",
                        "declared": margin,
                        "derived": derived_margin,
                    }
                )
        except (TypeError, ValueError):
            failures.append({"code": "bbb_baseline_noise_repeats_malformed"})
    if is_wgpu30_optimization and failures:
        return {
            "status": "failed",
            "baseline_bbb_commit": expected_baseline_commit,
            "baseline_bbb_report_sha256": expected_baseline_report_sha256,
            "baseline_bbb_binary_sha256": expected_baseline_binary_sha256,
            "candidate_bbb_commit": candidate_commit,
            "maximum_margin": MAX_BASELINE_FPS_MARGIN,
            "results": [],
            "stability": None,
            "failures": failures,
        }
    comparisons = evidence.get("comparisons")
    results: list[dict[str, object]] = []
    if not isinstance(comparisons, list):
        failures.append({"code": "bbb_candidate_comparisons_missing"})
    else:
        required = _candidate_required_scopes()
        observed: set[tuple[str, str]] = set()
        for entry in comparisons:
            if not isinstance(entry, Mapping):
                failures.append({"code": "bbb_candidate_comparison_malformed"})
                continue
            scope = (str(entry.get("scope_type")), str(entry.get("scope_id")))
            if scope in observed:
                failures.append(
                    {"code": "bbb_candidate_comparison_duplicate", "scope": list(scope)}
                )
            observed.add(scope)
            try:
                result = evaluate_candidate_fps_noninferiority(
                    float(entry["baseline_mean_fps"]),
                    float(entry["candidate_mean_fps"]),
                    comparison_margin,
                )
            except (KeyError, TypeError, ValueError, RetinaAcceptanceError) as error:
                failures.append(
                    {"code": "bbb_candidate_comparison_malformed", "scope": list(scope), "error": str(error)}
                )
                continue
            result.update({"scope_type": scope[0], "scope_id": scope[1]})
            results.append(result)
            if not result["passed"]:
                failures.append(
                    {"code": "bbb_candidate_fps_regression", "scope": list(scope)}
                )
        missing = sorted(required - observed)
        if missing:
            failures.append(
                {"code": "bbb_candidate_required_scopes_missing", "scopes": [list(item) for item in missing]}
            )
        unexpected = sorted(observed - required)
        if unexpected:
            failures.append(
                {
                    "code": "bbb_candidate_comparisons_unexpected",
                    "scopes": [list(item) for item in unexpected],
                }
            )
    stability = validate_candidate_stability_evidence(evidence, comparison_margin)
    failures.extend(stability["failures"])
    return {
        "status": "passed" if not failures else "failed",
        "baseline_bbb_commit": expected_baseline_commit,
        "baseline_bbb_report_sha256": expected_baseline_report_sha256,
        "baseline_bbb_binary_sha256": expected_baseline_binary_sha256,
        "candidate_bbb_commit": candidate_commit,
        "maximum_margin": MAX_BASELINE_FPS_MARGIN,
        "results": results,
        "stability": stability,
        "failures": failures,
    }


def validate_and_summarize(
    display: DisplayProbe,
    runs: Sequence[Mapping[str, object]],
    *,
    bbb_commit: str = BASELINE_BBB_COMMIT,
    candidate_fps_evidence: Mapping[str, object] | None = None,
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
        "stability_policy": {
            "policy": STABILITY_POLICY,
            "cv_role": "diagnostic-only-when-absolute-metrics-pass",
            "maximum_candidate_fps_margin": MAX_BASELINE_FPS_MARGIN,
        },
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
                {
                    "code": "workload_contract_mismatch",
                    "field": field,
                    "values": sorted(values),
                }
            )
    for index, run in enumerate(runs, start=1):
        if run.get("status") != "passed":
            reasons.append({"code": "run_not_passed", "run": index, "status": run.get("status")})
        if run.get("vsync") is not False or run.get("framerate_limit") != "unlimited":
            reasons.append({"code": "frame_pacing_contract_mismatch", "run": index})
        if run.get("effective_window_mode") != "fullscreen":
            reasons.append(
                {
                    "code": "window_mode_mismatch",
                    "run": index,
                    "client": run.get("client"),
                    "requested": "fullscreen",
                    "actual": run.get("effective_window_mode"),
                }
            )
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
        elif {
            str(profile.get("segment"))
            for profile in profiles
            if isinstance(profile, Mapping)
        } != set(CAPTURE_SEGMENTS):
            reasons.append({"code": "profile_segments_mismatch", "run": index})
    if reasons:
        return {
            **base,
            "status": "blocked",
            "formal_comparison_complete": False,
            "metrics_publishable": False,
            "blockers": reasons,
            "comparison": None,
            "stability_acceptance": None,
            "frame_phase_diagnostics": None,
            "frame_flight_diagnostics": None,
            "gpu_stage_diagnostics": None,
            "input_latency_diagnostics": None,
            "bbb_baseline_comparison": None,
        }
    try:
        (
            normalized_runs,
            sample_matrix,
            frame_phase_matrix,
            frame_flight_matrix,
            gpu_stage_matrix,
            input_latency_matrix,
        ) = _materialize_formal_samples(runs)
    except RetinaAcceptanceError as error:
        blocker = {"code": "raw_evidence_integrity_failure", "error": str(error)}
        return {
            **base,
            "status": "blocked",
            "formal_comparison_complete": False,
            "metrics_publishable": False,
            "blockers": [blocker],
            "comparison": None,
            "stability_acceptance": None,
            "frame_phase_diagnostics": None,
            "frame_flight_diagnostics": None,
            "gpu_stage_diagnostics": None,
            "input_latency_diagnostics": None,
            "bbb_baseline_comparison": None,
        }
    all_runs = tuple(range(1, len(normalized_runs) + 1))
    comparison = {
        client: {
            "application_frame_time": _scoped_summary(
                sample_matrix,
                normalized_runs,
                all_runs,
                client,
                "application_frame_time",
            ),
            "process_cpu_frame_time": _combined_process_cpu_summary(normalized_runs, client),
            "gpu_duration": _scoped_summary(
                sample_matrix, normalized_runs, all_runs, client, "gpu_duration"
            ),
        }
        for client in ("vanilla", "bbb")
    }
    stability = _build_stability_acceptance(sample_matrix, normalized_runs)
    frame_phase_diagnostics = _build_frame_phase_diagnostics(
        frame_phase_matrix, normalized_runs
    )
    frame_flight_diagnostics = _build_frame_flight_diagnostics(
        frame_flight_matrix, frame_phase_matrix, normalized_runs
    )
    gpu_stage_diagnostics = _build_gpu_stage_diagnostics(
        gpu_stage_matrix, normalized_runs
    )
    input_latency_diagnostics = _build_input_latency_diagnostics(
        input_latency_matrix, normalized_runs
    )
    if bbb_commit == BASELINE_BBB_COMMIT:
        baseline_comparison = {
            "role": "established-baseline",
            "status": "not-applicable",
            "baseline_bbb_commit": BASELINE_BBB_COMMIT,
            "baseline_bbb_report_sha256": BASELINE_BBB_REPORT_SHA256,
            "baseline_bbb_binary_sha256": BASELINE_BBB_BINARY_SHA256,
            "future_candidates_require_interleaved_pairs": True,
            "maximum_margin": MAX_BASELINE_FPS_MARGIN,
            "failures": [],
        }
    else:
        baseline_comparison = {
            "role": "candidate",
            **validate_candidate_fps_evidence(candidate_fps_evidence, bbb_commit),
        }
    performance_blockers = list(stability["failures"])
    performance_blockers.extend(baseline_comparison["failures"])
    return {
        **base,
        "runs": normalized_runs,
        "status": "passed" if not performance_blockers else "blocked",
        "formal_comparison_complete": True,
        "metrics_publishable": True,
        "blockers": performance_blockers,
        "comparison": comparison,
        "stability_acceptance": stability,
        "frame_phase_diagnostics": frame_phase_diagnostics,
        "frame_flight_diagnostics": frame_flight_diagnostics,
        "gpu_stage_diagnostics": gpu_stage_diagnostics,
        "input_latency_diagnostics": input_latency_diagnostics,
        "bbb_baseline_comparison": baseline_comparison,
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


def send_screenshot_input(backend: MacOsQuartzBackend, pid: int) -> None:
    _post_key_code(backend, F2_KEY_CODE, True)
    _post_key_code(backend, F2_KEY_CODE, False)


def send_profiler_input(backend: MacOsQuartzBackend, pid: int) -> None:
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
    complete: Callable[[Path], bool] | None = None,
) -> Path:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        candidates = set(root.glob(pattern)) - before
        if candidates:
            candidate = max(candidates, key=lambda path: path.stat().st_mtime_ns)
            if complete is None or complete(candidate):
                return candidate
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
        "--fullscreenWidth",
        str(display.panel_native_width),
        "--fullscreenHeight",
        str(display.panel_native_height),
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
    input_latency_diagnostics: bool = False,
) -> tuple[str, ...]:
    argv = build_bbb_launch_argv(
        binary=binary,
        vanilla_root=vanilla_root,
        server_port=server_port,
        control_port=control_port,
    )
    for option, value in (
        ("--width", str(display.panel_native_width)),
        ("--height", str(display.panel_native_height)),
        ("--client-vsync", "false"),
        ("--client-framerate-limit", str(UNLIMITED_FPS_SENTINEL)),
    ):
        argv = _replace_option(argv, option, value)
    result = (
        *argv,
        "--game-dir",
        str(game_dir),
        "--client-fullscreen",
        "true",
        "--client-fullscreen-resolution",
        fullscreen_video_mode(display),
    )
    if input_latency_diagnostics:
        result = (*result, "--renderer-input-latency-diagnostics")
    return result


def _adapter_from_log(client: str, text: str, fallback: str) -> dict[str, str]:
    if client == "bbb":
        name = re.search(
            r'adapter_name=(?:"([^"]+)"|(.+?))\s+adapter_backend=', text
        )
        backend = re.search(r'adapter_backend=(?:"([^"]+)"|([^ ]+))', text)
        return {
            "device": (name.group(1) or name.group(2)).strip() if name else fallback,
            "api": (backend.group(1) or backend.group(2)) if backend else "metal",
        }
    renderer = re.search(r"(?im)^\s*(?:OpenGL renderer|Renderer):\s*(.+)$", text)
    return {
        "device": renderer.group(1).strip() if renderer else fallback,
        "api": "opengl",
    }


def _surface_presentation_from_log(client: str, text: str) -> dict[str, object]:
    if client != "bbb":
        return {
            "api": "opengl",
            "present_mode": "device-controlled",
            "desired_maximum_frame_latency": None,
            "expected_metal_drawable_count": None,
            "actual_metal_drawable_count": None,
            "actual_drawable_pixels": None,
            "in_flight_submission_budget": None,
        }
    line = re.search(r"renderer surface presentation selected[^\n]*", text)
    if line is None:
        raise RetinaAcceptanceError("BBB log lacks surface presentation contract")
    line_text = line.group(0)

    def required_integer(name: str) -> int:
        match = re.search(rf"{name}=([0-9]+)", line_text)
        if match is None:
            raise RetinaAcceptanceError(f"BBB surface presentation contract lacks {name}")
        return int(match.group(1))

    present_mode_match = re.search(r"present_mode=([A-Za-z]+)", line_text)
    if present_mode_match is None:
        raise RetinaAcceptanceError("BBB surface presentation contract lacks present_mode")
    latency = required_integer("desired_maximum_frame_latency")
    expected_count_match = re.search(r"expected_metal_drawable_count=([0-9]+)", line_text)
    budget_match = re.search(r"in_flight_submission_budget=([0-9]+)", line_text)
    fence_strategy_match = re.search(
        r'submission_fence_strategy=(?:"([^"]+)"|([^ ]+))', line_text
    )
    has_wgpu30_contract = expected_count_match is not None or budget_match is not None
    if has_wgpu30_contract and (expected_count_match is None or budget_match is None):
        raise RetinaAcceptanceError("BBB wgpu 30 surface presentation contract is incomplete")
    observed = re.search(
        r"(?:native|external) Metal layer presentation observed[^\n]*"
        r"drawable_width=([0-9.]+)[^\n]*"
        r"drawable_height=([0-9.]+)[^\n]*maximum_drawable_count=([0-9]+)",
        text,
    )
    if has_wgpu30_contract and observed is None:
        raise RetinaAcceptanceError("BBB log lacks actual CAMetalLayer presentation evidence")
    drawable = (
        [round(float(observed.group(1))), round(float(observed.group(2)))]
        if observed is not None
        else None
    )
    return {
        "api": "metal",
        "present_mode": present_mode_match.group(1),
        "desired_maximum_frame_latency": latency,
        "expected_metal_drawable_count": (
            int(expected_count_match.group(1))
            if expected_count_match is not None
            else latency + 1
        ),
        "actual_metal_drawable_count": int(observed.group(3)) if observed is not None else None,
        "actual_drawable_pixels": drawable,
        "in_flight_submission_budget": (
            int(budget_match.group(1)) if budget_match is not None else None
        ),
        "submission_fence_strategy": (
            (fence_strategy_match.group(1) or fence_strategy_match.group(2))
            if fence_strategy_match is not None
            else "render-submission-fence-legacy-log"
        ),
        "count_basis": (
            "runtime CAMetalLayer maximumDrawableCount observation"
            if observed is not None
            else "legacy configured expectation; not actual evidence"
        ),
    }


def _runtime_dependency_receipt_from_log(text: str) -> dict[str, str]:
    line = re.search(r"renderer dependency receipt[^\n]*", text)
    if line is None:
        raise RetinaAcceptanceError("BBB log lacks runtime dependency receipt")

    def field(name: str) -> str:
        match = re.search(rf'{name}=(?:"([^"]+)"|([^ ]+))', line.group(0))
        if match is None:
            raise RetinaAcceptanceError(f"BBB runtime dependency receipt lacks {name}")
        return (match.group(1) or match.group(2)).strip()

    return {
        "wgpu": field("wgpu_version"),
        "wgpu-core": field("wgpu_core_version"),
        "wgpu-hal": field("wgpu_hal_version"),
        "wgpu-types": field("wgpu_types_version"),
        "naga": field("naga_version"),
        "upstream_metal_fence_commit": field("upstream_metal_fence_commit"),
        "upstream_metal_fence_strategy": field("upstream_metal_fence_strategy"),
    }


def _gpu_timestamp_strategy_from_log(text: str) -> str | None:
    line = re.search(r"renderer GPU timestamp strategy selected[^\n]*", text)
    if line is None:
        return None
    strategy = re.search(r"strategy=([A-Za-z]+)", line.group(0))
    if strategy is None:
        raise RetinaAcceptanceError("BBB GPU timestamp strategy log is malformed")
    return strategy.group(1)


def load_bbb_build_info(binary: Path) -> dict[str, object]:
    completed = subprocess.run(
        (str(binary), "--build-info"),
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise RetinaAcceptanceError(
            f"BBB --build-info failed with exit {completed.returncode}: {completed.stderr.strip()}"
        )
    try:
        value = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RetinaAcceptanceError(f"BBB --build-info is not JSON: {error}") from error
    expected = {
        "wgpu_version": "30.0.0",
        "wgpu_core_version": "30.0.0",
        "wgpu_hal_version": "30.0.0",
        "wgpu_types_version": "30.0.0",
        "naga_version": "30.0.0",
        "upstream_metal_fence_commit": WGPU_METAL_FENCE_COMMIT,
        "upstream_metal_fence_strategy": "completion-handler-condition-variable",
    }
    mismatches = {
        field: {"expected": expected_value, "actual": value.get(field)}
        for field, expected_value in expected.items()
        if value.get(field) != expected_value
    }
    if mismatches:
        raise RetinaAcceptanceError(f"BBB --build-info dependency mismatch: {mismatches}")
    return value


def bbb_runtime_log_filter(*, require_gpu_stages: bool) -> str:
    filters = list(BBB_RUNTIME_LOG_FILTERS)
    if require_gpu_stages:
        filters.append(GPU_STAGE_RUNTIME_LOG_FILTER)
    return ",".join(filters)


def _write_profile_samples(
    path: Path,
    profile_path: Path,
    segment: str,
    cpu_nanos: int,
    *,
    require_frame_flight: bool,
    require_gpu_stages: bool,
    require_input_latency: bool,
) -> dict[str, object]:
    samples = profiler_samples(profile_path)
    entry = parse_profiler_archive(profile_path)
    entry["segment"] = segment
    for metric in ("application_frame_time", "gpu_duration", "gpu_duration_scope"):
        milliseconds = samples[metric]
        output = path / f"{segment}-{metric}-samples.json"
        output.write_bytes(canonical_json_bytes({"milliseconds": milliseconds}))
        entry[f"{metric}_samples"] = str(output)
    frame_phases = samples.get("frame_phases")
    if isinstance(frame_phases, Mapping):
        output = path / f"{segment}-frame-phases-samples.json"
        output.write_bytes(canonical_json_bytes({"milliseconds": frame_phases}))
        entry["frame_phase_samples"] = str(output)
    frame_flight = samples.get("frame_flight")
    if isinstance(frame_flight, Mapping):
        output = path / f"{segment}-frame-flight-samples.json"
        output.write_bytes(canonical_json_bytes({"values": frame_flight}))
        entry["frame_flight_samples"] = str(output)
    elif require_frame_flight:
        raise RetinaAcceptanceError(
            f"BBB profiler archive lacks required {FRAME_FLIGHT_SCHEMA} evidence"
        )
    gpu_stages = samples.get("gpu_stages")
    if isinstance(gpu_stages, Mapping):
        output = path / f"{segment}-gpu-stages-samples.json"
        output.write_bytes(canonical_json_bytes({"milliseconds": gpu_stages}))
        entry["gpu_stage_samples"] = str(output)
    elif require_gpu_stages:
        raise RetinaAcceptanceError(
            f"BBB profiler archive lacks required {GPU_STAGE_SCHEMA} evidence"
        )
    input_latency = samples.get("input_latency")
    if isinstance(input_latency, Mapping):
        output = path / f"{segment}-input-latency-samples.json"
        output.write_bytes(canonical_json_bytes({"values": input_latency}))
        entry["input_latency_samples"] = str(output)
    elif require_input_latency:
        raise RetinaAcceptanceError(
            f"BBB profiler archive lacks required {INPUT_LATENCY_SCHEMA} evidence"
        )
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
    required_frame_phase_schema: str | None = FRAME_PHASE_SCHEMA,
    require_frame_flight: bool = False,
    require_gpu_stages: bool = False,
    require_input_latency: bool = False,
) -> dict[str, object]:
    if client not in {"vanilla", "bbb"}:
        raise RetinaAcceptanceError(f"unsupported formal client {client!r}")
    if required_frame_phase_schema not in (None, *FRAME_PHASE_CONTRACTS):
        raise RetinaAcceptanceError("unknown required frame phase schema")
    require_unlocked_macos_console()
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
            input_latency_diagnostics=require_input_latency,
        )
        profile_root = artifact_dir / "debug" / "profiling"
    screenshot_root = game_dir / "screenshots"
    settings = settings_identity()
    input_script = {
        "events": ["F2", "F3-down", "L-down", "L-up", "F3-up"],
        "at_markers": [f"{segment}:capture_start" for segment in CAPTURE_SEGMENTS],
        "targeting": "CoreGraphics keyboard events posted directly to exact client pid",
    }
    result: dict[str, object] = {
        "schema": SCHEMA,
        "profile": PROFILE_ID,
        "client": client,
        "status": "started",
        "requested_window_mode": "fullscreen",
        "effective_window_mode": "pending",
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
        "required_frame_phase_schema": (
            required_frame_phase_schema if client == "bbb" else None
        ),
        "required_frame_flight_schema": (
            FRAME_FLIGHT_SCHEMA if client == "bbb" and require_frame_flight else None
        ),
        "required_gpu_stage_schema": (
            GPU_STAGE_SCHEMA if client == "bbb" and require_gpu_stages else None
        ),
        "required_input_latency_schema": (
            INPUT_LATENCY_SCHEMA if client == "bbb" and require_input_latency else None
        ),
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
                    "RUST_LOG": bbb_runtime_log_filter(
                        require_gpu_stages=client == "bbb" and require_gpu_stages
                    ),
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
                send_screenshot_input(input_backend, client_process.pid)
                screenshot = _wait_for_new_file(
                    screenshot_root,
                    "*.png",
                    before_screenshots,
                    process=client_process,
                    timeout=10.0,
                    complete=png_file_complete,
                )
                dimensions = png_dimensions(screenshot)
                result["screenshots"].append(
                    {
                        "segment": segment,
                        "path": str(screenshot),
                        "sha256": file_digest(screenshot, "sha256"),
                        "pixels": list(dimensions),
                    }
                )
                if dimensions != display.target_drawable:
                    result["drawable_pixels"] = list(dimensions)
                    target_width, target_height = display.target_drawable
                    raise RetinaAcceptanceError(
                        f"{client} drawable {dimensions[0]}x{dimensions[1]} differs from "
                        f"target {target_width}x{target_height}"
                    )
                cpu_before = process_cpu_time_nanos(client_process.pid)
                send_profiler_input(input_backend, client_process.pid)
                profile = _wait_for_new_file(
                    profile_root,
                    "*.zip",
                    before_profiles,
                    process=client_process,
                    timeout=PROFILE_SECONDS + 10.0,
                    complete=zipfile.is_zipfile,
                )
                cpu_after = process_cpu_time_nanos(client_process.pid)
                result["profiles"].append(
                    _write_profile_samples(
                        artifact_dir,
                        profile,
                        segment,
                        cpu_after - cpu_before,
                        require_frame_flight=require_frame_flight,
                        require_gpu_stages=require_gpu_stages,
                        require_input_latency=require_input_latency,
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
        try:
            result["surface_presentation"] = _surface_presentation_from_log(client, client_text)
        except RetinaAcceptanceError as error:
            result["surface_presentation"] = None
            result.setdefault("validation_failures", []).append(str(error))
            result["status"] = "failed"
        if client == "bbb":
            try:
                if "renderer dependency receipt" in client_text:
                    result["runtime_dependency_receipt"] = _runtime_dependency_receipt_from_log(
                        client_text
                    )
                    load_committed_wgpu_dependency_receipt()
                    result["committed_dependency_receipt"] = {
                        "path": str(WGPU_DEPENDENCY_RECEIPT_PATH),
                        "sha256": file_digest(WGPU_DEPENDENCY_RECEIPT_PATH, "sha256"),
                    }
                else:
                    result["runtime_dependency_receipt"] = None
                result["gpu_timestamp_strategy"] = _gpu_timestamp_strategy_from_log(
                    client_text
                )
            except RetinaAcceptanceError as error:
                result.setdefault("validation_failures", []).append(str(error))
                result["status"] = "failed"
        result["effective_window_mode"] = (
            "windowed-fallback"
            if client == "vanilla"
            and "Failed to find suitable monitor for fullscreen mode" in client_text
            else "fullscreen"
        )
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
    candidate_fps_evidence_path: Path | None = None,
) -> dict[str, object]:
    if artifact_root.exists() and any(artifact_root.iterdir()):
        raise RetinaAcceptanceError(f"formal artifact root is not empty: {artifact_root}")
    capture_commit = require_clean_full_gated_head()
    dependency_receipt = load_committed_wgpu_dependency_receipt()
    bbb_build_info = load_bbb_build_info(bbb_binary)
    candidate_fps_evidence = None
    if capture_commit != BASELINE_BBB_COMMIT:
        if candidate_fps_evidence_path is None:
            raise RetinaAcceptanceError(
                "candidate formal ABBA requires predeclared interleaved BBB baseline/candidate A/B evidence"
            )
        candidate_fps_evidence = _load_object(
            candidate_fps_evidence_path, "BBB baseline/candidate A/B evidence"
        )
        candidate_check = validate_candidate_fps_evidence(
            candidate_fps_evidence, capture_commit
        )
        if candidate_check["status"] != "passed":
            codes = ", ".join(
                str(item.get("code")) for item in candidate_check["failures"]
            )
            raise RetinaAcceptanceError(f"candidate BBB A/B evidence is invalid: {codes}")
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
            require_frame_flight=client == "bbb",
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
                    "bbb_build_info": bbb_build_info,
                    "wgpu_dependency_receipt_sha256": file_digest(
                        WGPU_DEPENDENCY_RECEIPT_PATH, "sha256"
                    ),
                    "runs": run_paths,
                }
            )
        )
        if result.get("status") != "passed" or result.get("drawable_pixels") != list(
            display.target_drawable
        ):
            break
    report = validate_and_summarize(
        display,
        runs,
        bbb_commit=capture_commit,
        candidate_fps_evidence=candidate_fps_evidence,
    )
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
            "bbb_build_info": bbb_build_info,
            "wgpu_dependency_receipt": dependency_receipt,
            "wgpu_dependency_receipt_path": str(WGPU_DEPENDENCY_RECEIPT_PATH),
            "wgpu_dependency_receipt_sha256": file_digest(
                WGPU_DEPENDENCY_RECEIPT_PATH, "sha256"
            ),
        }
    )
    report_path = artifact_root / "report.json"
    report_path.write_bytes(canonical_json_bytes(report))
    return report


def _detect_command(args: argparse.Namespace) -> int:
    display = probe_builtin_display()
    profile_contract = load_profile_contract()
    dependency_receipt = load_committed_wgpu_dependency_receipt()
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
        "wgpu_dependency_receipt": dependency_receipt,
        "wgpu_dependency_receipt_path": str(WGPU_DEPENDENCY_RECEIPT_PATH),
        "wgpu_dependency_receipt_sha256": file_digest(
            WGPU_DEPENDENCY_RECEIPT_PATH, "sha256"
        ),
    }
    payload = canonical_json_bytes(report)
    if args.output is None:
        sys.stdout.buffer.write(payload)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(payload)
    return 0


def _dependency_receipt_command(args: argparse.Namespace) -> int:
    receipt = load_committed_wgpu_dependency_receipt()
    payload = canonical_json_bytes(
        {
            "receipt": receipt,
            "path": str(WGPU_DEPENDENCY_RECEIPT_PATH),
            "sha256": file_digest(WGPU_DEPENDENCY_RECEIPT_PATH, "sha256"),
        }
    )
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
    bbb_commit = str(manifest.get("bbb_commit", BASELINE_BBB_COMMIT))
    candidate_fps_evidence = (
        _load_object(args.bbb_baseline_ab_report, "BBB baseline/candidate A/B evidence")
        if args.bbb_baseline_ab_report is not None
        else None
    )
    report = validate_and_summarize(
        display,
        runs,
        bbb_commit=bbb_commit,
        candidate_fps_evidence=candidate_fps_evidence,
    )
    report["manifest"] = str(args.manifest)
    report["manifest_sha256"] = hashlib.sha256(args.manifest.read_bytes()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_json_bytes(report))
    return 0 if report["status"] == "passed" else 2


def _revalidate_command(args: argparse.Namespace) -> int:
    source = _load_object(args.report, "existing macos-native-retina report")
    display_value = source.get("display")
    runs = source.get("runs")
    if not isinstance(display_value, dict) or not isinstance(runs, list):
        raise RetinaAcceptanceError("existing report lacks display or runs")
    source_sha256 = file_digest(args.report, "sha256")
    bbb_commit = str(source.get("bbb_commit", ""))
    candidate_fps_evidence = (
        _load_object(args.bbb_baseline_ab_report, "BBB baseline/candidate A/B evidence")
        if args.bbb_baseline_ab_report is not None
        else None
    )
    report = validate_and_summarize(
        DisplayProbe(**display_value),
        runs,
        bbb_commit=bbb_commit,
        candidate_fps_evidence=candidate_fps_evidence,
    )
    for field in (
        "artifact_root",
        "bbb_binary",
        "bbb_binary_sha256",
        "bbb_commit",
        "formal_manifest",
        "formal_manifest_sha256",
        "replay",
        "replay_sha256",
        "world_snapshot",
    ):
        if field in source:
            report[field] = source[field]
    report["capture_profile_contract"] = source.get("profile_contract")
    report["capture_profile_contract_sha256"] = source.get("profile_contract_sha256")
    report["profile_contract"] = load_profile_contract()
    report["profile_contract_path"] = str(PROFILE_CONTRACT_PATH)
    report["profile_contract_sha256"] = file_digest(PROFILE_CONTRACT_PATH, "sha256")
    report["validator"] = {
        "path": str(Path(__file__).resolve()),
        "sha256": file_digest(Path(__file__).resolve(), "sha256"),
        "focused_tests": str(
            Path(__file__).resolve().parent / "tests" / "test_macos_native_retina.py"
        ),
        "focused_tests_sha256": file_digest(
            Path(__file__).resolve().parent / "tests" / "test_macos_native_retina.py",
            "sha256",
        ),
    }
    report["supersedes_report_sha256"] = source_sha256
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_json_bytes(report))
    return 0 if report["status"] == "passed" else 2


def _candidate_check_command(args: argparse.Namespace) -> int:
    evidence = _load_object(args.report, "BBB baseline/candidate A/B evidence")
    result = validate_candidate_fps_evidence(evidence, args.candidate_commit)
    result["evidence_report"] = str(args.report)
    result["evidence_report_sha256"] = file_digest(args.report, "sha256")
    payload = canonical_json_bytes(result)
    if args.output is None:
        sys.stdout.buffer.write(payload)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(payload)
    return 0 if result["status"] == "passed" else 2


def _run_command(args: argparse.Namespace) -> int:
    report = run_formal(
        artifact_root=args.artifact_root,
        vanilla_root=args.vanilla_root,
        java_path=args.java,
        bbb_binary=args.bbb_binary,
        replay_path=args.replay,
        timeout=args.timeout,
        candidate_fps_evidence_path=args.bbb_baseline_ab_report,
    )
    return 0 if report["status"] == "passed" else 2


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    detect = subparsers.add_parser("detect")
    detect.add_argument("--output", type=Path)
    detect.add_argument("--server-port", type=int, default=25565)
    detect.add_argument("--control-port", type=int, default=24454)
    dependency_receipt = subparsers.add_parser("dependency-receipt")
    dependency_receipt.add_argument("--output", type=Path)
    summarize = subparsers.add_parser("summarize")
    summarize.add_argument("--manifest", type=Path, required=True)
    summarize.add_argument("--output", type=Path, required=True)
    summarize.add_argument("--bbb-baseline-ab-report", type=Path)
    revalidate = subparsers.add_parser("revalidate")
    revalidate.add_argument("--report", type=Path, required=True)
    revalidate.add_argument("--output", type=Path, required=True)
    revalidate.add_argument("--bbb-baseline-ab-report", type=Path)
    candidate_check = subparsers.add_parser("candidate-check")
    candidate_check.add_argument("--report", type=Path, required=True)
    candidate_check.add_argument("--candidate-commit", required=True)
    candidate_check.add_argument("--output", type=Path)
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
    run.add_argument("--bbb-baseline-ab-report", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        if args.command == "detect":
            return _detect_command(args)
        if args.command == "dependency-receipt":
            return _dependency_receipt_command(args)
        if args.command == "summarize":
            return _summarize_command(args)
        if args.command == "revalidate":
            return _revalidate_command(args)
        if args.command == "candidate-check":
            return _candidate_check_command(args)
        if args.command == "run":
            return _run_command(args)
        raise AssertionError(args.command)
    except RetinaAcceptanceError as error:
        print(f"macos-native-retina: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
