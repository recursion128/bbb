"""Machine contract for the formal Vanilla/BBB host-audio receipt."""

from __future__ import annotations

import hashlib
import json
import re
from collections.abc import Mapping
from pathlib import Path
from typing import Any

AUDIO_RECEIPT_SCHEMA = 2
AUDIO_MECHANISM_IDS = ("audio-sound-manager-engine",)
BBB_AUDIO_TEST_IDS = (
    "openal::tests::verified_openal_library_executes_both_hrtf_context_branches",
    "openal::tests::verified_openal_device_tracker_executes_callback_or_polling_selection",
    "openal::tests::verified_openal_channel_executes_static_and_four_buffer_stream_owners",
    "openal::tests::verified_openal_sources_restore_their_context_across_device_reload",
    "runtime::tests::stopped_openal_channel_returns_pool_slot_while_instance_shell_remains",
    "device::tests::verified_device_tracker_runs_selected_mode_and_forced_refresh",
)
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")


def _has_sha256(value: object) -> bool:
    return isinstance(value, str) and _SHA256.fullmatch(value) is not None


def canonical_sha256(value: object) -> str:
    payload = (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _file_identity_reason(label: str, value: object) -> str | None:
    if not isinstance(value, Mapping):
        return f"audio receipt lacks {label} file identity"
    path_value = value.get("path")
    digest = value.get("sha256")
    if not isinstance(path_value, str) or not _has_sha256(digest):
        return f"audio receipt has malformed {label} file identity"
    path = Path(path_value)
    if not path.is_file() or _file_sha256(path) != digest:
        return f"audio receipt {label} file identity is stale"
    return None


def _bbb_test_run_passed(item: Mapping[str, Any], binary_path: object) -> bool:
    argv = item.get("argv")
    stdout = item.get("stdout")
    stderr = item.get("stderr")
    return (
        item.get("status") == "passed"
        and item.get("returncode") == 0
        and isinstance(argv, list)
        and argv[:1] == [binary_path]
        and argv[1:]
        == [item.get("test_id"), "--exact", "--ignored", "--nocapture"]
        and isinstance(stdout, Mapping)
        and _has_sha256(stdout.get("sha256"))
        and isinstance(stderr, Mapping)
        and _has_sha256(stderr.get("sha256"))
    )


def formal_audio_receipt_reasons(value: Mapping[str, Any]) -> list[str]:
    """Return fail-closed reasons for an alleged formal host-device receipt."""

    reasons: list[str] = []
    if value.get("schema") != AUDIO_RECEIPT_SCHEMA:
        reasons.append(
            f"audio differential receipt schema is not {AUDIO_RECEIPT_SCHEMA}"
        )
    if value.get("mechanism_ids") != list(AUDIO_MECHANISM_IDS):
        reasons.append("audio receipt Mechanism-ID mapping is incomplete")
    if value.get("driver") not in {"default", "alsa"}:
        reasons.append("formal audio receipt does not use a real host driver")
    if value.get("device_evidence") != "host-device":
        reasons.append("formal audio receipt lacks host-device evidence")
    if value.get("evidence_class") != "formal":
        reasons.append("audio receipt evidence class is not formal")
    if value.get("valid") is not True or value.get("formal_valid") is not True:
        reasons.append("audio differential receipt did not pass")

    vanilla = value.get("vanilla")
    observations = value.get("observations")
    if not isinstance(vanilla, Mapping):
        reasons.append("audio receipt lacks Vanilla owner identity")
    if (
        not isinstance(observations, list)
        or len(observations) != 2
        or [item.get("requested_hrtf") for item in observations if isinstance(item, Mapping)]
        != [False, True]
        or any(
            not isinstance(item, Mapping) or item.get("status") != "passed"
            for item in observations
        )
    ):
        reasons.append("audio receipt lacks both passing Vanilla HRTF branches")
    if not isinstance(value.get("frozen_contract"), Mapping):
        reasons.append("audio receipt lacks the validated Vanilla pool/HRTF contract")

    bbb = value.get("bbb")
    if not isinstance(bbb, Mapping):
        reasons.append("audio receipt lacks BBB executable evidence")
        return reasons
    if bbb.get("valid") is not True:
        reasons.append("BBB audio executable contract did not pass")
    binary = bbb.get("test_binary")
    if not isinstance(binary, Mapping) or not _has_sha256(binary.get("sha256")):
        reasons.append("BBB audio test binary identity is missing")
    elif (
        value.get("client_binary") != binary.get("path")
        or value.get("client_binary_sha256") != binary.get("sha256")
    ):
        reasons.append("BBB audio top-level binary identity diverges from its test binary")
    build = binary.get("build") if isinstance(binary, Mapping) else None
    build_argv = build.get("argv") if isinstance(build, Mapping) else None
    if (
        not isinstance(build, Mapping)
        or build.get("returncode") != 0
        or not isinstance(build_argv, list)
        or not {
            "bbb-audio",
            "--features",
            "device",
            "--no-run",
            "--message-format=json",
        }.issubset(
            {str(part) for part in build_argv}
        )
    ):
        reasons.append("BBB audio test binary build contract is missing")
    openal = bbb.get("openal_library")
    if not isinstance(openal, Mapping) or not _has_sha256(openal.get("sha256")):
        reasons.append("BBB audio OpenAL library identity is missing")
    sound = bbb.get("sound_asset")
    if (
        not isinstance(sound, Mapping)
        or not _has_sha256(sound.get("sha256"))
        or not isinstance(sound.get("sha1"), str)
    ):
        reasons.append("BBB audio Vanilla sound identity is missing")

    runs = bbb.get("tests")
    by_id = {
        item.get("test_id"): item
        for item in runs
        if isinstance(item, Mapping) and isinstance(item.get("test_id"), str)
    } if isinstance(runs, list) else {}
    if set(by_id) != set(BBB_AUDIO_TEST_IDS) or len(by_id) != len(BBB_AUDIO_TEST_IDS):
        reasons.append("BBB audio receipt does not cover the exact executable test set")
    elif any(
        not _bbb_test_run_passed(item, binary.get("path"))
        for item in by_id.values()
    ):
        reasons.append("BBB audio receipt contains a failed or unhashed executable test")

    differential = value.get("differential")
    bbb_contract = bbb.get("contract")
    expected_bbb_contract = {
        "test_ids": list(BBB_AUDIO_TEST_IDS),
        "test_binary_sha256": binary.get("sha256") if isinstance(binary, Mapping) else None,
        "openal_library_sha256": openal.get("sha256") if isinstance(openal, Mapping) else None,
        "sound_asset_sha256": sound.get("sha256") if isinstance(sound, Mapping) else None,
    }
    if bbb_contract != expected_bbb_contract:
        reasons.append("BBB audio executable contract identity is stale")
    if (
        not isinstance(differential, Mapping)
        or differential.get("status") != "passed"
        or differential.get("vanilla_contract_sha256")
        != canonical_sha256(value.get("frozen_contract"))
        or differential.get("bbb_test_contract_sha256")
        != canonical_sha256(bbb_contract)
    ):
        reasons.append("audio receipt lacks a passing Vanilla/BBB differential contract")
    return reasons


def audio_receipt_file_reasons(value: Mapping[str, Any]) -> list[str]:
    """Verify every external file identity carried by a schema-2 audio receipt."""

    identities: list[tuple[str, object]] = []
    for index, observation in enumerate(value.get("observations", [])):
        if isinstance(observation, Mapping):
            identities.extend(
                (
                    (f"Vanilla HRTF[{index}] stdout", observation.get("stdout")),
                    (f"Vanilla HRTF[{index}] stderr", observation.get("stderr")),
                )
            )
    bbb = value.get("bbb")
    if isinstance(bbb, Mapping):
        binary = bbb.get("test_binary")
        identities.extend(
            (
                ("BBB test binary", binary),
                ("BBB OpenAL library", bbb.get("openal_library")),
                ("BBB Vanilla sound", bbb.get("sound_asset")),
            )
        )
        if isinstance(binary, Mapping) and isinstance(binary.get("build"), Mapping):
            identities.extend(
                (
                    ("BBB build stdout", binary["build"].get("stdout")),
                    ("BBB build stderr", binary["build"].get("stderr")),
                )
            )
        for index, test in enumerate(bbb.get("tests", [])):
            if isinstance(test, Mapping):
                identities.extend(
                    (
                        (f"BBB test[{index}] stdout", test.get("stdout")),
                        (f"BBB test[{index}] stderr", test.get("stderr")),
                    )
                )
    return [
        reason
        for label, identity in identities
        if (reason := _file_identity_reason(label, identity)) is not None
    ]
