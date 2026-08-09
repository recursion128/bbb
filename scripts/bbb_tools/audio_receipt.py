"""Machine contract for the formal Vanilla/BBB host-audio receipt."""

from __future__ import annotations

import hashlib
import json
import re
from collections.abc import Mapping
from pathlib import Path
from typing import Any

AUDIO_RECEIPT_SCHEMA = 3
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
_ALSA_CARD_ID = re.compile(r"[A-Za-z0-9_-]+\Z")
_PCI_ADDRESS = re.compile(
    r"(?:[0-9a-f]{4}:)?[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]\Z"
)
_PCI_ID = re.compile(r"0x[0-9a-f]{4}\Z")
ALSA_DMIX_IPC_KEY = 1111638596


def _has_sha256(value: object) -> bool:
    return isinstance(value, str) and _SHA256.fullmatch(value) is not None


def canonical_sha256(value: object) -> str:
    payload = (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def alsa_pci_config_text(card_id: str, device_index: int) -> str:
    """Return the exact plug -> dmix -> PCI-hardware ALSA capture route."""

    if _ALSA_CARD_ID.fullmatch(card_id) is None:
        raise ValueError(f"invalid ALSA card id: {card_id!r}")
    if device_index < 0:
        raise ValueError("ALSA device index cannot be negative")
    return (
        "</usr/share/alsa/alsa.conf>\n"
        "\n"
        "pcm.bbb_formal_hardware_dmix {\n"
        "    type dmix\n"
        f"    ipc_key {ALSA_DMIX_IPC_KEY}\n"
        "    ipc_perm 0660\n"
        "    slave {\n"
        f'        pcm "hw:{card_id},{device_index}"\n'
        "        format S32_LE\n"
        "        rate 48000\n"
        "        channels 2\n"
        "        period_size 1024\n"
        "        buffer_size 16384\n"
        "    }\n"
        "}\n"
        "\n"
        "pcm.!default {\n"
        "    type plug\n"
        '    slave.pcm "bbb_formal_hardware_dmix"\n'
        "}\n"
        "\n"
        "ctl.!default {\n"
        "    type hw\n"
        f"    card {card_id}\n"
        "}\n"
    )


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
    try:
        current = _file_sha256(path) if path.is_file() else None
    except OSError:
        current = None
    if current != digest:
        return f"audio receipt {label} file identity is stale"
    return None


def _verified_text(value: object) -> str | None:
    if _file_identity_reason("verified text", value) is not None:
        return None
    assert isinstance(value, Mapping)
    try:
        return Path(value["path"]).read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
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


def _formal_alsa_endpoint_reasons(value: object) -> list[str]:
    reasons: list[str] = []
    if not isinstance(value, Mapping):
        return ["formal audio receipt lacks an ALSA PCI endpoint contract"]
    if value.get("schema") != 1 or value.get("kind") != "alsa-pci-pcm":
        reasons.append("formal audio receipt ALSA endpoint kind is invalid")
    card_index = value.get("card_index")
    card_id = value.get("card_id")
    device_index = value.get("device_index")
    if (
        not isinstance(card_index, int)
        or isinstance(card_index, bool)
        or card_index < 0
        or not isinstance(device_index, int)
        or isinstance(device_index, bool)
        or device_index < 0
        or not isinstance(card_id, str)
        or _ALSA_CARD_ID.fullmatch(card_id) is None
    ):
        reasons.append("formal audio receipt ALSA endpoint indexes/id are malformed")

    pcm_node = value.get("pcm_node")
    if (
        not isinstance(pcm_node, Mapping)
        or pcm_node.get("path")
        != f"/dev/snd/pcmC{card_index}D{device_index}p"
        or pcm_node.get("character_device") is not True
        or pcm_node.get("read_write") is not True
        or not isinstance(pcm_node.get("major"), int)
        or isinstance(pcm_node.get("major"), bool)
        or pcm_node.get("major") < 0
        or not isinstance(pcm_node.get("minor"), int)
        or isinstance(pcm_node.get("minor"), bool)
        or pcm_node.get("minor") < 0
    ):
        reasons.append("formal audio receipt lacks an accessible ALSA PCM character device")

    proc = value.get("proc")
    pcm_info = proc.get("pcm_info") if isinstance(proc, Mapping) else None
    eld_snapshots = proc.get("eld_snapshots") if isinstance(proc, Mapping) else None
    if (
        not isinstance(proc, Mapping)
        or proc.get("card_id") != card_id
        or not isinstance(pcm_info, Mapping)
        or pcm_info.get("card") != str(card_index)
        or pcm_info.get("device") != str(device_index)
        or pcm_info.get("stream") != "PLAYBACK"
        or not isinstance(pcm_info.get("name"), str)
        or not pcm_info.get("name")
        or not isinstance(eld_snapshots, list)
    ):
        reasons.append("formal audio receipt ALSA proc identity is inconsistent")

    sysfs = value.get("sysfs")
    if (
        not isinstance(sysfs, Mapping)
        or not isinstance(sysfs.get("pci_address"), str)
        or _PCI_ADDRESS.fullmatch(sysfs.get("pci_address", "")) is None
        or any(
            not isinstance(sysfs.get(key), str)
            or _PCI_ID.fullmatch(sysfs.get(key, "")) is None
            for key in ("vendor", "device", "subsystem_vendor", "subsystem_device")
        )
        or not isinstance(sysfs.get("driver"), str)
        or not sysfs.get("driver")
        or not isinstance(pcm_node, Mapping)
        or sysfs.get("pcm_dev")
        != f"{pcm_node.get('major')}:{pcm_node.get('minor')}"
    ):
        reasons.append("formal audio receipt lacks a consistent PCI/sysfs backing device")

    config = value.get("alsa_config")
    expected_config_sha256 = None
    if isinstance(card_id, str) and isinstance(device_index, int):
        try:
            expected_config_sha256 = hashlib.sha256(
                alsa_pci_config_text(card_id, device_index).encode("utf-8")
            ).hexdigest()
        except ValueError:
            pass
    if (
        not isinstance(config, Mapping)
        or config.get("sha256") != expected_config_sha256
        or config.get("route") != ["plug", "dmix", "hw"]
        or config.get("default_pcm") != "default"
        or config.get("dmix_pcm") != "bbb_formal_hardware_dmix"
        or config.get("hardware_pcm") != f"hw:{card_id},{device_index}"
        or config.get("ipc_key") != ALSA_DMIX_IPC_KEY
        or not isinstance(config.get("base_config"), Mapping)
        or config["base_config"].get("path") != "/usr/share/alsa/alsa.conf"
        or not _has_sha256(config["base_config"].get("sha256"))
    ):
        reasons.append("formal audio receipt ALSA plug/dmix/hardware route is invalid")

    controlled = value.get("controlled_environment")
    if (
        not isinstance(controlled, Mapping)
        or not isinstance(config, Mapping)
        or controlled
        != {
            "ALSOFT_DRIVERS": "alsa",
            "ALSOFT_LOGLEVEL": "3",
            "ALSA_CONFIG_PATH": config.get("path"),
        }
    ):
        reasons.append("formal audio receipt did not freeze the OpenAL/ALSA environment")

    contract = dict(value)
    contract.pop("contract_sha256", None)
    if value.get("contract_sha256") != canonical_sha256(contract):
        reasons.append("formal audio receipt ALSA endpoint contract hash is stale")
    return reasons


def formal_audio_receipt_reasons(value: Mapping[str, Any]) -> list[str]:
    """Return fail-closed reasons for an alleged formal host-device receipt."""

    reasons: list[str] = []
    if value.get("schema") != AUDIO_RECEIPT_SCHEMA:
        reasons.append(
            f"audio differential receipt schema is not {AUDIO_RECEIPT_SCHEMA}"
        )
    if value.get("mechanism_ids") != list(AUDIO_MECHANISM_IDS):
        reasons.append("audio receipt Mechanism-ID mapping is incomplete")
    if value.get("driver") != "alsa":
        reasons.append("formal audio receipt does not use the ALSA host driver")
    if value.get("device_evidence") != "alsa-pci-pcm":
        reasons.append("formal audio receipt lacks ALSA PCI hardware evidence")
    reasons.extend(_formal_alsa_endpoint_reasons(value.get("alsa_endpoint")))
    endpoint = value.get("alsa_endpoint")
    expected_environment = (
        endpoint.get("controlled_environment")
        if isinstance(endpoint, Mapping)
        else None
    )
    if value.get("capture_environment") != expected_environment:
        reasons.append("audio receipt top-level capture environment diverges from its endpoint")
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
    frozen_contract = value.get("frozen_contract")
    if not isinstance(frozen_contract, Mapping):
        reasons.append("audio receipt lacks the validated Vanilla pool/HRTF contract")
    elif (
        not isinstance(frozen_contract.get("current_device"), str)
        or not frozen_contract.get("current_device")
        or not isinstance(observations, list)
        or any(
            not isinstance(item, Mapping)
            or item.get("current_device") != frozen_contract.get("current_device")
            for item in observations
        )
    ):
        reasons.append("audio receipt Vanilla observations do not match the frozen device")

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
    """Verify every external file identity carried by a schema-3 audio receipt."""

    identities: list[tuple[str, object]] = []
    observations = value.get("observations")
    if not isinstance(observations, list):
        observations = []
    for index, observation in enumerate(observations):
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
        tests = bbb.get("tests")
        if not isinstance(tests, list):
            tests = []
        for index, test in enumerate(tests):
            if isinstance(test, Mapping):
                identities.extend(
                    (
                        (f"BBB test[{index}] stdout", test.get("stdout")),
                        (f"BBB test[{index}] stderr", test.get("stderr")),
                    )
                )
    endpoint = value.get("alsa_endpoint")
    if isinstance(endpoint, Mapping):
        config = endpoint.get("alsa_config")
        proc = endpoint.get("proc")
        sysfs = endpoint.get("sysfs")
        identities.extend(
            (
                ("ALSA generated config", config),
                (
                    "ALSA base config",
                    config.get("base_config") if isinstance(config, Mapping) else None,
                ),
                (
                    "ALSA card-id snapshot",
                    proc.get("card_id_snapshot") if isinstance(proc, Mapping) else None,
                ),
                (
                    "ALSA PCM-info snapshot",
                    proc.get("pcm_info_snapshot") if isinstance(proc, Mapping) else None,
                ),
                (
                    "ALSA PCI sysfs snapshot",
                    sysfs.get("snapshot") if isinstance(sysfs, Mapping) else None,
                ),
            )
        )
        if isinstance(proc, Mapping):
            identities.extend(
                (f"ALSA ELD[{index}] snapshot", identity)
                for index, identity in enumerate(proc.get("eld_snapshots", []))
            )
    reasons = [
        reason
        for label, identity in identities
        if (reason := _file_identity_reason(label, identity)) is not None
    ]
    for index, observation in enumerate(observations):
        if not isinstance(observation, Mapping):
            continue
        stdout_payload = _verified_text(observation.get("stdout"))
        if stdout_payload is None:
            continue
        raw_objects = []
        for line in stdout_payload.splitlines():
            if not line.strip().startswith("{"):
                continue
            try:
                raw_value = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(raw_value, Mapping):
                raw_objects.append(dict(raw_value))
        expected = {
            key: field_value
            for key, field_value in observation.items()
            if key not in {"process_returncode", "stdout", "stderr"}
        }
        if raw_objects != [expected]:
            reasons.append(
                f"audio receipt Vanilla HRTF[{index}] stdout disagrees with its observation"
            )
    if isinstance(bbb, Mapping):
        tests = bbb.get("tests")
        if not isinstance(tests, list):
            tests = []
        for index, test in enumerate(tests):
            if not isinstance(test, Mapping):
                continue
            stdout_payload = _verified_text(test.get("stdout"))
            test_id = test.get("test_id")
            if stdout_payload is None or not isinstance(test_id, str):
                continue
            if not all(
                marker in stdout_payload
                for marker in (
                    "running 1 test",
                    f"test {test_id} ... ok",
                    "test result: ok. 1 passed; 0 failed",
                )
            ):
                reasons.append(
                    f"audio receipt BBB test[{index}] stdout lacks its exact pass markers"
                )
    if isinstance(endpoint, Mapping):
        proc = endpoint.get("proc")
        sysfs = endpoint.get("sysfs")
        if isinstance(proc, Mapping):
            card_snapshot = proc.get("card_id_snapshot")
            if (
                _file_identity_reason("ALSA card-id snapshot", card_snapshot) is None
                and isinstance(card_snapshot, Mapping)
            ):
                try:
                    card_snapshot_value = (
                        Path(card_snapshot["path"])
                        .read_text(encoding="utf-8")
                        .strip()
                    )
                except (OSError, UnicodeDecodeError):
                    card_snapshot_value = None
                if card_snapshot_value != proc.get("card_id"):
                    reasons.append(
                        "audio receipt ALSA card-id snapshot disagrees with its contract"
                    )
            pcm_snapshot = proc.get("pcm_info_snapshot")
            if (
                _file_identity_reason("ALSA PCM-info snapshot", pcm_snapshot) is None
                and isinstance(pcm_snapshot, Mapping)
            ):
                snapshot_fields = {}
                try:
                    snapshot_payload = Path(pcm_snapshot["path"]).read_text(
                        encoding="utf-8"
                    )
                except (OSError, UnicodeDecodeError):
                    snapshot_payload = ""
                for line in snapshot_payload.splitlines():
                    key, separator, field_value = line.partition(":")
                    if separator:
                        snapshot_fields[key.strip()] = field_value.strip()
                if snapshot_fields != proc.get("pcm_info"):
                    reasons.append(
                        "audio receipt ALSA PCM-info snapshot disagrees with its contract"
                    )
        if isinstance(sysfs, Mapping):
            pci_snapshot = sysfs.get("snapshot")
            if (
                _file_identity_reason("ALSA PCI sysfs snapshot", pci_snapshot) is None
                and isinstance(pci_snapshot, Mapping)
            ):
                try:
                    snapshot_value = json.loads(
                        Path(pci_snapshot["path"]).read_text(encoding="utf-8")
                    )
                except (OSError, UnicodeDecodeError, json.JSONDecodeError):
                    snapshot_value = None
                expected = dict(sysfs)
                expected.pop("snapshot", None)
                if snapshot_value != expected:
                    reasons.append(
                        "audio receipt ALSA PCI sysfs snapshot disagrees with its contract"
                    )
    return reasons
