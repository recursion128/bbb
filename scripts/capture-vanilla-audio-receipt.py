#!/usr/bin/env python3
"""Capture executable Vanilla 26.1 audio pool, HRTF, and lifecycle facts."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
from dataclasses import asdict
from pathlib import Path
from typing import Any, Mapping

from bbb_tools.audio_receipt import (
    AUDIO_MECHANISM_IDS,
    AUDIO_RECEIPT_SCHEMA,
    BBB_AUDIO_TEST_IDS,
)
from bbb_tools.vanilla.arguments import (
    add_vanilla_runtime,
    resolve_runtime_inputs,
    resolve_workspace,
)
from official_client_benchmark import (
    audit_distribution,
    canonical_json_bytes,
    file_digest,
    safe_extract_natives,
)


REPO_ROOT = Path(__file__).resolve().parents[1]
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/VanillaAudioParityReport.java"
PROBE_CLASS = "VanillaAudioParityReport"
SOURCE_OWNERS = (
    "com/mojang/blaze3d/audio/Library.java",
    "net/minecraft/client/sounds/ChannelAccess.java",
    "net/minecraft/client/sounds/SoundEngine.java",
)
CHANNEL_DEBUG = re.compile(r"Sounds: 0/(?P<static>[0-9]+) \+ 0/(?P<streaming>[0-9]+)\Z")
BBB_AUDIO_MANIFEST = REPO_ROOT / "crates/bbb-audio/Cargo.toml"
BBB_SOUND_ASSET = "minecraft/sounds/ambient/cave/cave1.ogg"


class AudioReceiptFailure(RuntimeError):
    """The official owner could not produce a valid audio observation."""


def repo_commit_identity(repo: Path, *, require_clean: bool) -> str:
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        check=False,
        text=True,
        capture_output=True,
    )
    if head.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", head.stdout.strip()) is None:
        raise AudioReceiptFailure("audio receipt requires an exact repository commit")
    status = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=repo,
        check=False,
        text=True,
        capture_output=True,
    )
    if status.returncode != 0:
        raise AudioReceiptFailure("audio receipt could not inspect repository state")
    if require_clean and status.stdout.strip():
        raise AudioReceiptFailure("formal audio receipt requires a clean worktree")
    return head.stdout.strip()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_runtime(parser)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument(
        "--driver",
        choices=("default", "alsa", "null"),
        default="default",
        help="OpenAL Soft output driver; null is diagnostic-only evidence",
    )
    args = parser.parse_args()
    resolve_runtime_inputs(args)
    return args


def parse_probe_output(output: str) -> dict[str, Any]:
    objects: list[dict[str, Any]] = []
    for line in output.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            objects.append(value)
    if len(objects) != 1:
        raise AudioReceiptFailure(
            f"expected one JSON object from Vanilla audio probe, got {len(objects)}"
        )
    return objects[0]


def vanilla_channel_limits(mono_sources: int) -> tuple[int, int]:
    if mono_sources < 0:
        raise AudioReceiptFailure("Vanilla mono source count cannot be negative")
    streaming = min(max(int(mono_sources**0.5), 2), 8)
    static = min(max(mono_sources - streaming, 8), 255)
    return static, streaming


def validate_lifecycle(value: Any) -> None:
    if not isinstance(value, Mapping):
        raise AudioReceiptFailure("Vanilla lifecycle observation is missing")
    if value.get("minimum_source_lifetime_ticks") != 20:
        raise AudioReceiptFailure("Vanilla minimum source lifetime is not 20 ticks")
    trace = value.get("trace")
    if not isinstance(trace, list) or len(trace) != 21:
        raise AudioReceiptFailure("Vanilla lifecycle trace must cover ticks 0..20")
    for tick, observation in enumerate(trace):
        if not isinstance(observation, Mapping) or observation.get("tick") != tick:
            raise AudioReceiptFailure(f"Vanilla lifecycle trace is invalid at tick {tick}")
        retained = tick < 20
        for key in ("is_active", "channel_retained", "delete_time_retained"):
            if observation.get(key) is not retained:
                raise AudioReceiptFailure(
                    f"Vanilla lifecycle {key} is invalid at tick {tick}"
                )


def validate_probe_pair(
    disabled: Mapping[str, Any], enabled: Mapping[str, Any]
) -> dict[str, Any]:
    for requested, observation in ((False, disabled), (True, enabled)):
        if observation.get("schema") != 1:
            raise AudioReceiptFailure("Vanilla audio probe schema must be 1")
        if observation.get("minecraft_version") != "26.1":
            raise AudioReceiptFailure("Vanilla audio probe version must be 26.1")
        if observation.get("requested_hrtf") is not requested:
            raise AudioReceiptFailure("Vanilla audio probe HRTF branch is mislabeled")
        if observation.get("status") != "passed":
            error = observation.get("error") or "unknown OpenAL failure"
            raise AudioReceiptFailure(f"Vanilla audio probe failed: {error}")
        validate_lifecycle(observation.get("lifecycle"))

    if disabled.get("current_device") != enabled.get("current_device"):
        raise AudioReceiptFailure("HRTF branches opened different OpenAL devices")
    if disabled.get("mono_sources") != enabled.get("mono_sources"):
        raise AudioReceiptFailure("HRTF branches observed different source capacities")
    mono_sources = disabled.get("mono_sources")
    if not isinstance(mono_sources, int) or isinstance(mono_sources, bool):
        raise AudioReceiptFailure("Vanilla mono source count is invalid")
    expected_static, expected_streaming = vanilla_channel_limits(mono_sources)

    for observation in (disabled, enabled):
        debug = observation.get("channel_debug")
        match = CHANNEL_DEBUG.fullmatch(debug) if isinstance(debug, str) else None
        if match is None:
            raise AudioReceiptFailure(f"invalid Vanilla channel debug string: {debug!r}")
        actual = (int(match["static"]), int(match["streaming"]))
        if actual != (expected_static, expected_streaming):
            raise AudioReceiptFailure(
                "Vanilla channel pools do not match Library's device-derived formula"
            )

    if disabled.get("hrtf_status") != 0:
        raise AudioReceiptFailure("Vanilla HRTF-disabled context did not report disabled")
    hrtf_supported = bool(enabled.get("hrtf_extension")) and (
        isinstance(enabled.get("hrtf_specifiers"), int)
        and enabled["hrtf_specifiers"] > 0
    )
    expected_enabled_status = 1 if hrtf_supported else 0
    if enabled.get("hrtf_status") != expected_enabled_status:
        raise AudioReceiptFailure(
            "Vanilla HRTF-enabled context did not use the supported fallback branch"
        )
    return {
        "mono_sources": mono_sources,
        "static_capacity": expected_static,
        "streaming_capacity": expected_streaming,
        "hrtf_supported": hrtf_supported,
        "hrtf_disabled_status": disabled["hrtf_status"],
        "hrtf_enabled_status": enabled["hrtf_status"],
        "minimum_source_lifetime_ticks": 20,
    }


def java_version(java: Path) -> str:
    result = subprocess.run(
        [str(java), "-version"], check=True, text=True, capture_output=True
    )
    return (result.stderr or result.stdout).strip()


def host_audio_access(device_root: Path = Path("/dev/snd")) -> dict[str, Any]:
    nodes = []
    if device_root.is_dir():
        for path in sorted(device_root.iterdir()):
            try:
                metadata = path.stat()
            except OSError as error:
                nodes.append({"path": str(path), "error": str(error)})
                continue
            if not stat.S_ISCHR(metadata.st_mode):
                continue
            nodes.append(
                {
                    "path": str(path),
                    "mode": stat.filemode(metadata.st_mode),
                    "uid": metadata.st_uid,
                    "gid": metadata.st_gid,
                    "read_write": os.access(path, os.R_OK | os.W_OK),
                }
            )
    return {
        "uid": os.getuid(),
        "gid": os.getgid(),
        "groups": os.getgroups(),
        "device_root": str(device_root),
        "device_root_present": device_root.is_dir(),
        "device_nodes": nodes,
        "accessible_device_nodes": sum(bool(node.get("read_write")) for node in nodes),
    }


def parse_bbb_test_binary(output: str) -> Path:
    executables: set[Path] = set()
    for line in output.splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (
            not isinstance(value, dict)
            or value.get("reason") != "compiler-artifact"
            or value.get("manifest_path") != str(BBB_AUDIO_MANIFEST)
            or not isinstance(value.get("profile"), dict)
            or value["profile"].get("test") is not True
            or not isinstance(value.get("executable"), str)
        ):
            continue
        target = value.get("target")
        if isinstance(target, dict) and target.get("name") == "bbb_audio":
            executables.add(Path(value["executable"]).resolve())
    if len(executables) != 1:
        raise AudioReceiptFailure(
            f"expected one bbb-audio test binary from Cargo, got {len(executables)}"
        )
    executable = executables.pop()
    if not executable.is_file() or not os.access(executable, os.X_OK):
        raise AudioReceiptFailure(f"bbb-audio test binary is unavailable: {executable}")
    return executable


def build_bbb_test_binary(artifact_dir: Path) -> dict[str, Any]:
    argv = [
        str(REPO_ROOT / "scripts/cargo-dev.sh"),
        "test",
        "-p",
        "bbb-audio",
        "--features",
        "device",
        "--no-run",
        "--message-format=json",
    ]
    result = subprocess.run(
        argv, cwd=REPO_ROOT, check=False, text=True, capture_output=True
    )
    stdout_path = artifact_dir / "bbb-build.stdout"
    stderr_path = artifact_dir / "bbb-build.stderr"
    stdout_path.write_text(result.stdout, encoding="utf-8")
    stderr_path.write_text(result.stderr, encoding="utf-8")
    if result.returncode != 0:
        raise AudioReceiptFailure(
            f"bbb-audio test build failed; see {stderr_path.resolve()}"
        )
    executable = parse_bbb_test_binary(result.stdout)
    return {
        "path": str(executable),
        "sha256": file_digest(executable, "sha256"),
        "build": {
            "argv": argv,
            "returncode": result.returncode,
            "stdout": {
                "path": str(stdout_path.resolve()),
                "sha256": file_digest(stdout_path, "sha256"),
            },
            "stderr": {
                "path": str(stderr_path.resolve()),
                "sha256": file_digest(stderr_path, "sha256"),
            },
        },
    }


def verified_openal_library(natives: tuple[Any, ...]) -> dict[str, Any]:
    matches = [Path(entry.path) for entry in natives if Path(entry.path).name == "libopenal.so"]
    if len(matches) != 1:
        raise AudioReceiptFailure(
            f"expected one verified Vanilla libopenal.so, got {len(matches)}"
        )
    path = matches[0].resolve()
    return {
        "path": str(path),
        "sha1": file_digest(path, "sha1"),
        "sha256": file_digest(path, "sha256"),
        "size": path.stat().st_size,
    }


def verified_bbb_sound_asset(workspace: Any, asset_index_id: str) -> dict[str, Any]:
    index_path = workspace.indexed_assets / "indexes" / f"{asset_index_id}.json"
    try:
        index = json.loads(index_path.read_text(encoding="utf-8"))
        entry = index["objects"][BBB_SOUND_ASSET]
        digest = entry["hash"]
        size = entry["size"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise AudioReceiptFailure(
            f"Vanilla sound asset index is malformed for {BBB_SOUND_ASSET}: {error}"
        ) from error
    if (
        not isinstance(digest, str)
        or re.fullmatch(r"[0-9a-f]{40}", digest) is None
        or not isinstance(size, int)
        or size < 0
    ):
        raise AudioReceiptFailure(f"Vanilla sound identity is malformed: {BBB_SOUND_ASSET}")
    path = workspace.indexed_assets / "objects" / digest[:2] / digest
    if (
        not path.is_file()
        or path.stat().st_size != size
        or file_digest(path, "sha1") != digest
    ):
        raise AudioReceiptFailure(f"Vanilla sound object is stale: {BBB_SOUND_ASSET}")
    return {
        "logical_path": BBB_SOUND_ASSET,
        "path": str(path.resolve()),
        "sha1": digest,
        "sha256": file_digest(path, "sha256"),
        "size": size,
    }


def run_bbb_audio_tests(
    *,
    binary: Path,
    openal_library: Path,
    sound_asset: Path,
    driver: str,
    artifact_dir: Path,
) -> list[dict[str, Any]]:
    environment = os.environ.copy()
    environment["BBB_TEST_OPENAL_LIBRARY"] = str(openal_library)
    environment["BBB_TEST_OPENAL_OGG"] = str(sound_asset)
    if driver == "default":
        environment.pop("ALSOFT_DRIVERS", None)
    else:
        environment["ALSOFT_DRIVERS"] = driver
    runs = []
    for index, test_id in enumerate(BBB_AUDIO_TEST_IDS, start=1):
        argv = [str(binary), test_id, "--exact", "--ignored", "--nocapture"]
        result = subprocess.run(
            argv, check=False, text=True, capture_output=True, env=environment
        )
        label = test_id.rsplit("::", 1)[-1]
        stdout_path = artifact_dir / f"bbb-{index:02d}-{label}.stdout"
        stderr_path = artifact_dir / f"bbb-{index:02d}-{label}.stderr"
        stdout_path.write_text(result.stdout, encoding="utf-8")
        stderr_path.write_text(result.stderr, encoding="utf-8")
        exact_marker = f"test {test_id} ... ok"
        passed = (
            result.returncode == 0
            and "running 1 test" in result.stdout
            and exact_marker in result.stdout
            and "test result: ok. 1 passed; 0 failed" in result.stdout
        )
        runs.append(
            {
                "test_id": test_id,
                "argv": argv,
                "returncode": result.returncode,
                "status": "passed" if passed else "failed",
                "stdout": {
                    "path": str(stdout_path.resolve()),
                    "sha256": file_digest(stdout_path, "sha256"),
                },
                "stderr": {
                    "path": str(stderr_path.resolve()),
                    "sha256": file_digest(stderr_path, "sha256"),
                },
            }
        )
    return runs


def run_probe(
    *,
    java: Path,
    classes: Path,
    classpath: str,
    natives: Path,
    requested_hrtf: bool,
    driver: str,
    artifact_dir: Path,
) -> dict[str, Any]:
    label = "hrtf-enabled" if requested_hrtf else "hrtf-disabled"
    argv = [
        str(java),
        "--enable-native-access=ALL-UNNAMED",
        f"-Djava.library.path={natives}",
        "-cp",
        os.pathsep.join((str(classes), classpath)),
        PROBE_CLASS,
        str(requested_hrtf).lower(),
    ]
    environment = os.environ.copy()
    if driver == "default":
        environment.pop("ALSOFT_DRIVERS", None)
    else:
        environment["ALSOFT_DRIVERS"] = driver
    result = subprocess.run(argv, check=False, text=True, capture_output=True, env=environment)
    stdout_path = artifact_dir / f"{label}.stdout"
    stderr_path = artifact_dir / f"{label}.stderr"
    stdout_path.write_text(result.stdout, encoding="utf-8")
    stderr_path.write_text(result.stderr, encoding="utf-8")
    try:
        observation = parse_probe_output(result.stdout)
    except AudioReceiptFailure as error:
        observation = {
            "schema": 1,
            "minecraft_version": "26.1",
            "requested_hrtf": requested_hrtf,
            "status": "failed",
            "error": str(error),
        }
    observation["process_returncode"] = result.returncode
    observation["stdout"] = {
        "path": str(stdout_path.resolve()),
        "sha256": file_digest(stdout_path, "sha256"),
    }
    observation["stderr"] = {
        "path": str(stderr_path.resolve()),
        "sha256": file_digest(stderr_path, "sha256"),
    }
    return observation


def capture(args: argparse.Namespace) -> tuple[Path, dict[str, Any]]:
    bbb_commit = repo_commit_identity(REPO_ROOT, require_clean=args.driver != "null")
    artifact_dir = args.artifact_dir.resolve()
    if artifact_dir.exists() and any(artifact_dir.iterdir()):
        raise AudioReceiptFailure(
            f"audio receipt artifact directory must be empty: {artifact_dir}"
        )
    artifact_dir.mkdir(parents=True, exist_ok=True)
    workspace = resolve_workspace(args)
    audit = audit_distribution(
        workspace.root, java_path=args.java, verify_asset_objects=False
    )
    natives_dir = artifact_dir / "natives"
    natives = safe_extract_natives(
        (Path(entry.path) for entry in audit.native_classifiers), natives_dir
    )
    openal_library = verified_openal_library(natives)
    sound_asset = verified_bbb_sound_asset(workspace, audit.asset_index_id)
    test_binary = build_bbb_test_binary(artifact_dir)
    classes_dir = artifact_dir / "classes"
    classes_dir.mkdir()
    classpath = os.pathsep.join(
        [entry.path for entry in audit.libraries] + [audit.client.path]
    )
    compile_result = subprocess.run(
        [
            str(args.javac),
            "-cp",
            classpath,
            "-d",
            str(classes_dir),
            str(PROBE_SOURCE),
        ],
        check=False,
        text=True,
        capture_output=True,
    )
    compile_stdout = artifact_dir / "javac.stdout"
    compile_stderr = artifact_dir / "javac.stderr"
    compile_stdout.write_text(compile_result.stdout, encoding="utf-8")
    compile_stderr.write_text(compile_result.stderr, encoding="utf-8")
    if compile_result.returncode != 0:
        raise AudioReceiptFailure(
            f"Vanilla audio probe compilation failed; see {compile_stderr}"
        )

    observations = [
        run_probe(
            java=args.java,
            classes=classes_dir,
            classpath=classpath,
            natives=natives_dir,
            requested_hrtf=requested,
            driver=args.driver,
            artifact_dir=artifact_dir,
        )
        for requested in (False, True)
    ]
    validation_error = None
    frozen_contract = None
    try:
        frozen_contract = validate_probe_pair(observations[0], observations[1])
    except AudioReceiptFailure as error:
        validation_error = str(error)

    bbb_tests = run_bbb_audio_tests(
        binary=Path(test_binary["path"]),
        openal_library=Path(openal_library["path"]),
        sound_asset=Path(sound_asset["path"]),
        driver=args.driver,
        artifact_dir=artifact_dir,
    )
    bbb_valid = all(test["status"] == "passed" for test in bbb_tests)
    bbb_contract = {
        "test_ids": list(BBB_AUDIO_TEST_IDS),
        "test_binary_sha256": test_binary["sha256"],
        "openal_library_sha256": openal_library["sha256"],
        "sound_asset_sha256": sound_asset["sha256"],
    }
    differential_valid = validation_error is None and bbb_valid
    vanilla_contract_sha256 = (
        hashlib.sha256(canonical_json_bytes(frozen_contract)).hexdigest()
        if frozen_contract is not None
        else None
    )
    bbb_contract_sha256 = hashlib.sha256(
        canonical_json_bytes(bbb_contract)
    ).hexdigest()

    source_owners = []
    for relative in SOURCE_OWNERS:
        path = workspace.sources / relative
        source_owners.append(
            {
                "path": relative,
                "sha256": file_digest(path, "sha256"),
            }
        )
    receipt = {
        "schema": AUDIO_RECEIPT_SCHEMA,
        "bbb_commit": bbb_commit,
        "mechanism_ids": list(AUDIO_MECHANISM_IDS),
        "minecraft_version": "26.1",
        "driver": args.driver,
        "device_evidence": "synthetic-null" if args.driver == "null" else "host-device",
        "evidence_class": (
            "formal" if differential_valid and args.driver != "null" else "diagnostic"
        ),
        "host_audio_access": host_audio_access(),
        "valid": differential_valid,
        "formal_valid": differential_valid and args.driver != "null",
        "invalid_reason": (
            validation_error
            if validation_error is not None
            else None if bbb_valid else "one or more BBB executable audio tests failed"
        ),
        "client_binary": test_binary["path"],
        "client_binary_sha256": test_binary["sha256"],
        "vanilla": {
            "client": asdict(audit.client),
            "metadata": asdict(audit.metadata),
            "source_owners": source_owners,
            "probe": {
                "path": str(PROBE_SOURCE.relative_to(REPO_ROOT)),
                "sha256": file_digest(PROBE_SOURCE, "sha256"),
            },
            "java": {
                "path": str(args.java.resolve()),
                "version": java_version(args.java),
            },
            "natives": [asdict(entry) for entry in natives],
        },
        "observations": observations,
        "frozen_contract": frozen_contract,
        "bbb": {
            "valid": bbb_valid,
            "test_binary": test_binary,
            "openal_library": openal_library,
            "sound_asset": sound_asset,
            "tests": bbb_tests,
            "contract": bbb_contract,
        },
        "differential": {
            "status": "passed" if differential_valid else "failed",
            "vanilla_contract_sha256": vanilla_contract_sha256,
            "bbb_test_contract_sha256": bbb_contract_sha256,
        },
    }
    output = artifact_dir / "vanilla-audio-receipt.json"
    output.write_bytes(canonical_json_bytes(receipt))
    return output, receipt


def main() -> int:
    args = parse_args()
    try:
        output, receipt = capture(args)
    except (AudioReceiptFailure, OSError, subprocess.SubprocessError) as error:
        print(f"vanilla audio receipt failed: {error}", file=sys.stderr)
        return 1
    print(output)
    if not receipt["valid"]:
        print(
            f"vanilla audio receipt invalid: {receipt['invalid_reason']}",
            file=sys.stderr,
        )
        return 1
    if not receipt["formal_valid"]:
        print(
            "vanilla audio receipt: synthetic null output is diagnostic only",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
