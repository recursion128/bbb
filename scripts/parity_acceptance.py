#!/usr/bin/env python3
"""Build and validate the Vanilla 26.1 mechanism acceptance report.

The parity matrix remains the only mechanism-status index.  This module joins
that index to the generated Vanilla owner inventory, the feature ledger, the
last full-gate receipt, and optional live/differential artifacts.  The output
is evidence, not a second hand-maintained checklist: every artifact and
scenario carries its reverse Mechanism-ID mapping and every mechanism record
is derived from the matrix row with which it must agree.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from functools import lru_cache
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from typing import Iterable

from bbb_tools.audio_receipt import (
    audio_receipt_file_reasons,
    formal_audio_receipt_reasons,
)
from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
from bbb_tools.executable_identity import (
    ELF_LOAD_IMAGE_ALGORITHM,
    ExecutableIdentityError,
    elf_load_image_identity,
)


REPO_ROOT = Path(__file__).resolve().parents[1]
MATRIX_PATH = Path("docs/wiki/parity/vanilla-26.1-matrix.md")
INVENTORY_PATH = Path(
    "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
)
LEDGER_PATH = Path("docs/unsupported-features.md")
REPORT_PATH = Path("docs/evidence/vanilla-26.1/parity-acceptance.json")
GATE_LOG_PATH = Path(".git/bbb/gate-log.tsv")
GATE_RECEIPT_DIGEST_KIND = "gate-receipt-line-v1"
SCHEMA_VERSION = 3
WORKFLOW_STAGES = (
    "implementation",
    "checkpoint",
    "code-freeze",
    "evidence-sweep",
    "final-performance",
    "final",
)
OPEN_REASON_CODES = frozenset(
    {
        "implementation-gap",
        "focused-evidence-pending",
        "formal-receipt-pending",
        "external-blocker",
    }
)
IMPLEMENTATION_BLOCKING_REASONS = frozenset(
    {"implementation-gap", "focused-evidence-pending"}
)
FORMAL_BINARY_IDENTITY_KINDS = frozenset(
    {"performance", "workload", "draw", "platform", "lifecycle", "audio"}
)
CLOSURE_METADATA_PREFIXES = ("docs/", "goal.md")
VALIDATION_ONLY_PATHS = frozenset(
    {
        "scripts/parity_acceptance.py",
        "scripts/tests/test_parity_acceptance.py",
        "scripts/tests/test_vanilla_audio_receipt.py",
    }
)
SCOPED_FORMAL_RECEIPT_REUSE_PATHS = VALIDATION_ONLY_PATHS | frozenset(
    {
        "scripts/capture-vanilla-audio-receipt.py",
        "scripts/bbb_tools/audio_receipt.py",
    }
)
OPEN_STATUSES = frozenset(
    {"partial", "missing", "active", "unverified", "deferred-external"}
)
ROUTED_STATUSES = OPEN_STATUSES | {"not-needed"}
REPOSITORY_WORKFLOW_MECHANISM_IDS = frozenset({"docs-chinese-mechanism-wiki"})
PROTOCOL_STATE_DOMAINS = frozenset(
    {
        "startup",
        "lifecycle",
        "protocol",
        "world",
        "movement",
        "input",
        "interaction",
    }
)
VISUAL_MECHANISMS = frozenset(
    {
        "rendering-block-model-pipeline",
        "rendering-camera-fluid-fog",
        "rendering-chunk-layer-routing",
        "rendering-fluid-mesh",
        "rendering-level-frame-graph",
        "rendering-level-scene-projection",
        "rendering-client-biome-tint",
        "rendering-block-entity-registration",
        "rendering-entity-renderer-registration",
        "rendering-entity-layer-registration",
    }
)
CENTRAL_PATH_VALIDATION_CLASSES = (
    (
        (
            "scripts/parity_acceptance.py",
            "scripts/docs_wiki_lint.py",
            "scripts/docs-lint.sh",
        ),
        frozenset({"acceptance-contract"}),
    ),
    (
        (
            MATRIX_PATH.as_posix(),
            INVENTORY_PATH.as_posix(),
            LEDGER_PATH.as_posix(),
        ),
        frozenset({"acceptance-contract"}),
    ),
    (("Cargo.toml", "Cargo.lock"), frozenset({"full-gate", "release-identity"})),
    (("crates/",), frozenset({"release-identity"})),
    (
        (
            "crates/bbb-native/src/runtime",
            "crates/bbb-native/src/simulation",
            "crates/bbb-native/src/main.rs",
        ),
        frozenset(
            {
                "protocol-state-receipt",
                "resource-platform-receipt",
                "visual-workload",
                "draw-evidence",
                "performance-abba",
            }
        ),
    ),
    (
        ("crates/bbb-renderer/", "crates/bbb-render-types/"),
        frozenset({"visual-workload", "draw-evidence", "performance-abba"}),
    ),
    (
        ("crates/bbb-protocol/", "crates/bbb-net/"),
        frozenset({"protocol-state-receipt", "resource-platform-receipt"}),
    ),
    (
        ("crates/bbb-world/",),
        frozenset(
            {
                "protocol-state-receipt",
                "visual-workload",
                "draw-evidence",
                "performance-abba",
            }
        ),
    ),
    (
        ("crates/bbb-pack/",),
        frozenset(
            {
                "resource-platform-receipt",
                "visual-workload",
                "draw-evidence",
                "audio-host-device",
                "performance-abba",
            }
        ),
    ),
    (
        (
            "scripts/official_client_benchmark.py",
            "scripts/official-client-benchmark.sh",
            "scripts/gpu_duration_capture.py",
            "scripts/fixtures/official-client-benchmark/",
        ),
        frozenset({"visual-workload", "draw-evidence", "performance-abba"}),
    ),
    (
        (
            "scripts/platform_acceptance.py",
            "scripts/platform-acceptance-gate.sh",
            "scripts/playable_gate.py",
            "scripts/playable-gate.sh",
            "scripts/fixtures/platform-acceptance/",
            "scripts/fixtures/playable-gate/",
        ),
        frozenset({"protocol-state-receipt", "resource-platform-receipt"}),
    ),
    (
        (
            "scripts/lifecycle_soak.py",
            "scripts/lifecycle-soak-gate.sh",
            "scripts/fixtures/lifecycle-soak/",
        ),
        frozenset({"protocol-state-receipt"}),
    ),
    (
        (
            "scripts/capture-vanilla-audio-receipt.py",
            "scripts/bbb_tools/audio_receipt.py",
        ),
        frozenset({"audio-host-device"}),
    ),
)
PROCESS_CAPTURE_TOOL_PATHS = (
    "scripts/process",
    "scripts/bbb_tools/execution/__init__.py",
    "scripts/bbb_tools/execution/cli.py",
    "scripts/bbb_tools/execution/deadline.py",
    "scripts/bbb_tools/execution/process.py",
)
PLAYABLE_CAPTURE_TOOL_PATHS = (
    "scripts/playable_gate.py",
    "scripts/playable-gate.sh",
    *PROCESS_CAPTURE_TOOL_PATHS,
)
PLATFORM_SEAT_CAPTURE_TOOL_PATHS = (
    "scripts/input",
    "scripts/bbb_tools/input/__init__.py",
    "scripts/bbb_tools/input/cli.py",
    "scripts/bbb_tools/input/contract.py",
    "scripts/bbb_tools/input/linux_external.py",
    "scripts/bbb_tools/input/macos_quartz.py",
    "scripts/seat_device.py",
    "scripts/seat_inject.py",
)
BENCHMARK_SHARED_TOOL_PATHS = (
    "scripts/bbb_tools/executable_identity.py",
    "scripts/bbb_tools/execution/deadline.py",
    "scripts/bbb_tools/execution/process.py",
)
CAPTURE_TOOL_PATHS = {
    "performance": (
        "scripts/official_client_benchmark.py",
        "scripts/official-client-benchmark.sh",
        "scripts/gpu_duration_capture.py",
        *BENCHMARK_SHARED_TOOL_PATHS,
    ),
    "workload": (
        "scripts/official_client_benchmark.py",
        "scripts/official-client-benchmark.sh",
        *BENCHMARK_SHARED_TOOL_PATHS,
    ),
    "draw": (
        "scripts/official_client_benchmark.py",
        "scripts/official-client-benchmark.sh",
        *BENCHMARK_SHARED_TOOL_PATHS,
    ),
    "platform": (
        "scripts/platform_acceptance.py",
        "scripts/platform-acceptance-gate.sh",
        *PLAYABLE_CAPTURE_TOOL_PATHS,
        *PLATFORM_SEAT_CAPTURE_TOOL_PATHS,
    ),
    "lifecycle": (
        "scripts/lifecycle_soak.py",
        "scripts/lifecycle-soak-gate.sh",
        *PLAYABLE_CAPTURE_TOOL_PATHS,
    ),
    "audio": (
        "scripts/capture-vanilla-audio-receipt.py",
        "scripts/bbb_tools/audio_receipt.py",
    ),
}
CAPTURE_INPUT_PATHS = {
    "performance": ("scripts/fixtures/official-client-benchmark/replay-26.1.json",),
    "workload": ("scripts/fixtures/official-client-benchmark/replay-26.1.json",),
    "draw": ("scripts/fixtures/official-client-benchmark/replay-26.1.json",),
    "platform": (
        "scripts/fixtures/platform-acceptance/linux-x86_64-wayland-vulkan.json",
        "scripts/fixtures/playable-gate/session-26.1.json",
        "scripts/fixtures/lifecycle-soak/manifest-26.1.json",
    ),
    "lifecycle": (
        "scripts/fixtures/lifecycle-soak/manifest-26.1.json",
        "scripts/fixtures/playable-gate/session-26.1.json",
    ),
}
CODE_REF_RE = re.compile(
    r"(?P<path>(?:crates|scripts|docs)/[A-Za-z0-9_./-]+(?:\.[A-Za-z0-9_-]+)?)"
    r"(?:#(?P<symbol>[A-Za-z0-9_:<>.-]+))?"
)
LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
CLASS_TOKEN_RE = re.compile(r"\b[A-Z][A-Za-z0-9_$]*\b")
RUST_TEST_RE = re.compile(
    r"(?P<attrs>(?:\s*#\[[^\]]+\])+)\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+"
    r"(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)\s*\("
)
RUST_EXECUTABLE_TEST_ATTR_RE = re.compile(
    r"#\[\s*(?:test|tokio::test)(?:\s*\([^\]]*\))?\s*\]"
)
PYTHON_TEST_RE = re.compile(r"^\s*(?:async\s+)?def\s+(test_[A-Za-z0-9_]+)\s*\(", re.MULTILINE)
MECHANISM_ID_DECLARATION_RE = re.compile(
    r"^机制\s+ID[:：]\s*`([a-z0-9]+(?:-[a-z0-9]+)*)`\s*$", re.MULTILINE
)


class AcceptanceError(RuntimeError):
    """The acceptance inputs or report violate the frozen contract."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def canonical_sha256(value: object) -> str:
    encoded = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return sha256_bytes(encoded)


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        raise AcceptanceError(
            f"git {' '.join(args)} failed: {result.stderr.strip()}"
        )
    return result.stdout.strip()


def current_head_and_clean(repo: Path, fallback_commit: str) -> tuple[str, bool]:
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if head.returncode != 0:
        return fallback_commit, True
    status = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    return head.stdout.strip(), status.returncode == 0 and not bool(status.stdout.strip())


def changed_repo_paths(repo: Path, commit: str) -> set[str]:
    """Return tracked and untracked paths not represented by ``commit``.

    Unit-test fixtures intentionally are not Git repositories, so absence of a
    repository means there is no commit-compatibility assertion to make there.
    Production generation always runs in the real checkout.
    """
    tracked = subprocess.run(
        ["git", "diff", "--name-only", commit, "--"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if tracked.returncode != 0:
        return set()
    untracked = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    values = set(tracked.stdout.splitlines())
    if untracked.returncode == 0:
        values.update(untracked.stdout.splitlines())
    return values


def workflow_stage_index(stage: str) -> int:
    try:
        return WORKFLOW_STAGES.index(stage)
    except ValueError as error:
        raise AcceptanceError(f"unknown parity workflow stage: {stage}") from error


def validation_classes(row: dict[str, str]) -> list[str]:
    """Derive validation work from the canonical matrix row.

    These classes are planning labels, not mechanism states.  They describe
    which shared receipts can close the row and let one domain sweep cover
    several Mechanism IDs without creating another checklist.
    """
    status = row["status"]
    mechanism_id = row["mechanism_id"]
    domain_parts = set(re.split(r"[/,]", row["domain"]))
    classes = {"vanilla-owner", "bbb-owner"}
    if status in {"not-needed", "deferred-external"}:
        classes.add("scope-ruling")
        return sorted(classes)
    classes.update({"focused-evidence", "full-gate"})
    if mechanism_id == "audio-sound-manager-engine":
        classes.add("audio-host-device")
    if mechanism_id.startswith("performance-"):
        classes.update(
            {"visual-workload", "draw-evidence", "performance-abba"}
        )
    if "rendering" in domain_parts or mechanism_id in VISUAL_MECHANISMS:
        classes.update({"visual-workload", "draw-evidence"})
    if domain_parts & PROTOCOL_STATE_DOMAINS:
        classes.add("protocol-state-receipt")
    if (
        domain_parts & {"resources", "platform", "persistence"}
        and mechanism_id != "audio-sound-manager-engine"
    ):
        classes.add("resource-platform-receipt")
    if classes & {
        "protocol-state-receipt",
        "resource-platform-receipt",
        "visual-workload",
        "draw-evidence",
        "audio-host-device",
        "performance-abba",
    }:
        classes.add("release-identity")
    return sorted(classes)


def hashed_repo_paths(
    repo: Path, roles_by_path: dict[str, set[str]]
) -> list[dict[str, object]]:
    values = []
    for relative, roles in sorted(roles_by_path.items()):
        path = repo / relative
        if not path.is_file():
            continue
        values.append(
            {
                "path": relative,
                "roles": sorted(roles),
                "sha256": sha256_file(path),
            }
        )
    return values


def path_matches_rule(path: str, token: str) -> bool:
    if token.endswith("/"):
        return path.startswith(token)
    return path == token or path.startswith(f"{token}/")


def is_closure_metadata_path(path: str) -> bool:
    return any(
        path.startswith(prefix) if prefix.endswith("/") else path == prefix
        for prefix in CLOSURE_METADATA_PREFIXES
    )


def direct_dependency_validation_classes(
    record: dict[str, object], dependency: dict[str, object]
) -> set[str]:
    roles = {
        str(role)
        for role in dependency.get("roles", [])
        if isinstance(role, str)
    }
    if "owner-implementation" in roles:
        return {
            str(value)
            for value in record.get("required_validation_classes", [])
        }
    if roles & {"focused-test", "focused-test-set", "owner-focused-test"}:
        return {"focused-evidence", "full-gate"}
    return {"acceptance-contract", "bbb-owner", "vanilla-owner"}


def artifact_input_validation_classes(
    record: dict[str, object], artifact_id: str
) -> set[str]:
    classes = {"acceptance-contract", "full-gate"}
    if artifact_id == "client-owner-inventory":
        classes.update({"bbb-owner", "vanilla-owner"})
    focused_tests = record.get("focused_tests", [])
    if isinstance(focused_tests, list) and any(
        artifact_id in test.get("source_artifact_ids", [])
        for test in focused_tests
        if isinstance(test, dict)
    ):
        classes.add("focused-evidence")
    return classes


def artifact_capture_validation_classes(
    record: dict[str, object], artifact_id: str
) -> set[str]:
    artifact_classes = {
        "audio-host-device": {"audio-host-device"},
        "draw-evidence": {"draw-evidence"},
        "lifecycle-soak": {"protocol-state-receipt"},
        "linux-platform": {
            "protocol-state-receipt",
            "resource-platform-receipt",
        },
        "performance-abba": {"performance-abba"},
        "visual-workload": {"visual-workload"},
    }.get(artifact_id, set())
    required = {
        str(value)
        for value in record.get("required_validation_classes", [])
        if isinstance(value, str)
    }
    return {"full-gate", *(artifact_classes & required)}


def change_impact_plan(
    records: list[dict[str, object]],
    changed_paths: Iterable[str],
    base_commit: str,
    artifacts: Iterable[dict[str, object]] = (),
) -> dict[str, object]:
    impacts: list[dict[str, object]] = []
    all_ids = sorted(
        str(record["mechanism_id"])
        for record in records
        if isinstance(record, dict) and isinstance(record.get("mechanism_id"), str)
    )
    records_by_id = {
        str(record["mechanism_id"]): record
        for record in records
        if isinstance(record, dict) and isinstance(record.get("mechanism_id"), str)
    }
    artifact_inputs: dict[str, list[tuple[str, str]]] = defaultdict(list)
    artifact_capture_paths: dict[str, list[tuple[str, str, str]]] = defaultdict(list)
    for artifact in artifacts:
        if not isinstance(artifact, dict) or not isinstance(
            artifact.get("artifact_id"), str
        ):
            continue
        artifact_id = str(artifact["artifact_id"])
        mechanism_ids = artifact.get("mechanism_ids", [])
        generator_inputs = artifact.get("generator_inputs", [])
        if not isinstance(mechanism_ids, list) or not isinstance(
            generator_inputs, list
        ):
            continue
        known_ids = [
            mechanism_id
            for mechanism_id in mechanism_ids
            if isinstance(mechanism_id, str) and mechanism_id in records_by_id
        ]
        for generator_input in generator_inputs:
            if not isinstance(generator_input, dict) or not isinstance(
                generator_input.get("path"), str
            ):
                continue
            for mechanism_id in known_ids:
                artifact_inputs[str(generator_input["path"])].append(
                    (artifact_id, mechanism_id)
                )
        kind = artifact.get("kind")
        if not isinstance(kind, str):
            continue
        capture_contract = artifact.get("capture_contract")
        tool_paths = set(CAPTURE_TOOL_PATHS.get(kind, ()))
        input_paths = set(CAPTURE_INPUT_PATHS.get(kind, ()))
        if isinstance(capture_contract, dict):
            for item in capture_contract.get("tools", []):
                if isinstance(item, dict) and isinstance(item.get("path"), str):
                    tool_paths.add(str(item["path"]))
            for item in capture_contract.get("inputs", []):
                if isinstance(item, dict) and isinstance(item.get("path"), str):
                    input_paths.add(str(item["path"]))
        for dependency_path, reason in (
            *((path, "capture-tool") for path in tool_paths),
            *((path, "capture-input") for path in input_paths),
        ):
            for mechanism_id in known_ids:
                artifact_capture_paths[dependency_path].append(
                    (artifact_id, mechanism_id, reason)
                )
    affected_ids: set[str] = set()
    required_classes: set[str] = set()
    for path in sorted(set(changed_paths)):
        path_ids: set[str] = set()
        path_classes: set[str] = set()
        reasons: set[str] = set()
        for record in records:
            if not isinstance(record, dict):
                continue
            mechanism_id = record.get("mechanism_id")
            bbb = record.get("bbb")
            if not isinstance(mechanism_id, str) or not isinstance(bbb, dict):
                continue
            closure = bbb.get("dependency_closure", [])
            matching_dependencies = [
                item
                for item in closure
                if isinstance(item, dict) and item.get("path") == path
            ]
            if matching_dependencies:
                path_ids.add(mechanism_id)
                for dependency in matching_dependencies:
                    path_classes.update(
                        direct_dependency_validation_classes(record, dependency)
                    )
                reasons.add("direct-dependency")
        for artifact_id, mechanism_id in artifact_inputs.get(path, []):
            record = records_by_id[mechanism_id]
            path_ids.add(mechanism_id)
            path_classes.update(
                artifact_input_validation_classes(record, artifact_id)
            )
            reasons.add("artifact-input")
        for artifact_id, mechanism_id, reason in artifact_capture_paths.get(path, []):
            record = records_by_id[mechanism_id]
            path_ids.add(mechanism_id)
            path_classes.update(
                artifact_capture_validation_classes(record, artifact_id)
            )
            reasons.add(reason)
        for tokens, classes in CENTRAL_PATH_VALIDATION_CLASSES:
            if not any(path_matches_rule(path, token) for token in tokens):
                continue
            reasons.add("central-owner")
            if "acceptance-contract" in classes:
                path_ids.update(all_ids)
                path_classes.update(classes)
                continue
            for record in records:
                if not isinstance(record, dict) or not isinstance(
                    record.get("mechanism_id"), str
                ):
                    continue
                record_classes = {
                    str(value)
                    for value in record.get("required_validation_classes", [])
                }
                shared = record_classes & set(classes)
                if shared:
                    path_ids.add(str(record["mechanism_id"]))
                    path_classes.update(shared)
        if path_ids:
            affected_ids.update(path_ids)
            required_classes.update(path_classes)
        impacts.append(
            {
                "mechanism_ids": sorted(path_ids),
                "path": path,
                "reasons": sorted(reasons) if path_ids else ["unrelated"],
                "required_validation_classes": sorted(path_classes),
            }
        )
    validation_only_changed_paths = sorted(
        str(impact["path"])
        for impact in impacts
        if str(impact["path"]) in VALIDATION_ONLY_PATHS
    )
    functional_impacts = [
        impact
        for impact in impacts
        if not is_closure_metadata_path(str(impact["path"]))
        and str(impact["path"]) not in VALIDATION_ONLY_PATHS
    ]
    return {
        "affected_mechanism_ids": sorted(affected_ids),
        "base_commit": base_commit,
        "changed_paths": sorted(set(changed_paths)),
        "functional_affected_mechanism_ids": sorted(
            {
                str(mechanism_id)
                for impact in functional_impacts
                for mechanism_id in impact["mechanism_ids"]
            }
        ),
        "functional_changed_paths": sorted(
            str(impact["path"]) for impact in functional_impacts
        ),
        "path_impacts": impacts,
        "required_validation_classes": sorted(required_classes),
        "validation_only_changed_paths": validation_only_changed_paths,
    }


def parity_semantic_contract_sha256(records: Iterable[dict[str, object]]) -> str:
    values = []
    for record in records:
        matrix = record.get("matrix")
        vanilla = record.get("vanilla")
        bbb = record.get("bbb")
        if not all(isinstance(value, dict) for value in (matrix, vanilla, bbb)):
            continue
        values.append(
            {
                "bbb_owner": bbb.get("owner"),
                "dependency_paths": [
                    {
                        "path": dependency.get("path"),
                        "roles": dependency.get("roles"),
                    }
                    for dependency in bbb.get("dependency_closure", [])
                    if isinstance(dependency, dict)
                ],
                "domain": matrix.get("domain"),
                "focused_test_contract": [
                    {
                        key: test.get(key)
                        for key in (
                            "command",
                            "evidence_set_count",
                            "evidence_set_sha256",
                            "ignored",
                            "reference",
                            "source_artifact_ids",
                        )
                        if key in test
                    }
                    for test in record.get("focused_tests", [])
                    if isinstance(test, dict)
                ],
                "inventory_owner_ids": vanilla.get("inventory_owner_ids"),
                "mechanism_id": record.get("mechanism_id"),
                "required_state_projection": matrix.get(
                    "required_state_projection"
                ),
                "required_validation_classes": record.get(
                    "required_validation_classes"
                ),
                "route_reopen_condition": matrix.get("route_reopen_condition"),
                "scope_ruling_sha256": record.get("scope_ruling_sha256"),
                "source_hashes": vanilla.get("source_hashes"),
                "vanilla_owner": vanilla.get("owner"),
            }
        )
    return canonical_sha256(values)


def implementation_blocking_ids(
    records: Iterable[dict[str, object]],
) -> list[str]:
    return sorted(
        str(record["mechanism_id"])
        for record in records
        if isinstance(record.get("matrix"), dict)
        and record["matrix"].get("status") != "deferred-external"
        and isinstance(record.get("open_reasons"), list)
        and IMPLEMENTATION_BLOCKING_REASONS
        & {str(reason) for reason in record["open_reasons"]}
    )


def parse_matrix(path: Path) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    seen: set[str] = set()
    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        if not line.startswith("| `"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 8:
            raise AcceptanceError(f"malformed matrix row at {path}:{line_number}")
        mechanism_id = cells[0].strip("`")
        if mechanism_id in seen:
            raise AcceptanceError(f"duplicate matrix mechanism {mechanism_id}")
        seen.add(mechanism_id)
        status_match = re.fullmatch(r"`([^`]+)`", cells[5])
        if status_match is None:
            raise AcceptanceError(f"malformed status for {mechanism_id}")
        rows.append(
            {
                "mechanism_id": mechanism_id,
                "domain": cells[1],
                "vanilla_owner": cells[2],
                "bbb_owner": cells[3],
                "required_state": cells[4],
                "status": status_match.group(1),
                "evidence": cells[6],
                "route": cells[7],
                "line": str(line_number),
            }
        )
    if not rows:
        raise AcceptanceError("parity matrix has no mechanism rows")
    return rows


def ledger_sections(text: str) -> tuple[dict[str, str], set[str]]:
    sections: list[str] = []
    current: list[str] = []
    for line in text.splitlines():
        if line.startswith("## ") and current:
            sections.append("\n".join(current))
            current = []
        current.append(line)
    if current:
        sections.append("\n".join(current))
    routed_sections: dict[str, list[str]] = defaultdict(list)
    for section in sections:
        for mechanism_id in re.findall(r"`([a-z0-9]+(?:-[a-z0-9]+)*)`", section):
            if section not in routed_sections[mechanism_id]:
                routed_sections[mechanism_id].append(section)
    by_id = {
        mechanism_id: "\n\n".join(values)
        for mechanism_id, values in routed_sections.items()
    }
    return by_id, set(by_id)


def source_index(source_root: Path) -> dict[str, list[Path]]:
    result: dict[str, list[Path]] = defaultdict(list)
    for path in source_root.rglob("*.java"):
        result[path.stem].append(path)
    for paths in result.values():
        paths.sort()
    return dict(result)


def relative_source(source_root: Path, path: Path) -> str:
    return path.relative_to(source_root).as_posix()


def matched_source_hashes(
    *,
    row: dict[str, str],
    owners: list[dict[str, object]],
    inventory_hashes: dict[str, str],
    class_sources: dict[str, list[Path]],
    source_root: Path,
    ledger_basis: str,
    evidence_basis: str,
) -> list[dict[str, object]]:
    paths: set[str] = set()
    for owner in owners:
        source = owner.get("source")
        if isinstance(source, dict) and isinstance(source.get("path"), str):
            paths.add(source["path"])

    def add_matching_tokens(text: str, maximum: int | None = None) -> None:
        for token in CLASS_TOKEN_RE.findall(text):
            if any(Path(relative).stem == token for relative in paths):
                continue
            for path in class_sources.get(token, []):
                paths.add(relative_source(source_root, path))
                if maximum is not None and len(paths) >= maximum:
                    return

    had_inventory_paths = bool(paths)
    # Inventory owners are the bounded census, while a matrix row may also name
    # a shared owner that is canonically inventoried under another mechanism.
    # Hash every explicit matrix owner in addition to this mechanism's census;
    # otherwise the presence of one inventory owner would hide shared owners.
    add_matching_tokens(row["vanilla_owner"])
    if not had_inventory_paths:
        add_matching_tokens(row["required_state"])
    if not paths:
        add_matching_tokens(evidence_basis, maximum=16)
    if not paths:
        add_matching_tokens(ledger_basis, maximum=16)
    result: list[dict[str, object]] = []
    for relative in sorted(paths):
        candidate = source_root / relative
        digest = inventory_hashes.get(relative)
        if candidate.is_file():
            actual = sha256_file(candidate)
            if digest is not None and digest != actual:
                raise AcceptanceError(f"stale inventory source hash: {relative}")
            digest = actual
        if digest is not None:
            result.append({"path": relative, "sha256": digest})
    return result


def mechanism_source_hashes(
    *,
    mechanism_id: str,
    row: dict[str, str],
    owners: list[dict[str, object]],
    inventory_hashes: dict[str, str],
    class_sources: dict[str, list[Path]],
    source_root: Path,
    ledger_basis: str,
    evidence_basis: str,
) -> list[dict[str, object]]:
    """Resolve Vanilla owner hashes without inventing owners for repo workflows."""
    if mechanism_id in REPOSITORY_WORKFLOW_MECHANISM_IDS:
        return []
    return matched_source_hashes(
        row=row,
        owners=owners,
        inventory_hashes=inventory_hashes,
        class_sources=class_sources,
        source_root=source_root,
        ledger_basis=ledger_basis,
        evidence_basis=evidence_basis,
    )


def allows_empty_vanilla_source_hashes(mechanism_id: str) -> bool:
    return mechanism_id in REPOSITORY_WORKFLOW_MECHANISM_IDS or mechanism_id.startswith(
        "coverage-"
    )


def resolve_markdown_link(repo: Path, source: Path, raw: str) -> Path | None:
    target = raw.strip().strip("<>").split("#", 1)[0]
    if not target or target.startswith(("http://", "https://", "mailto:")):
        return None
    candidate = (source.parent / target).resolve()
    try:
        candidate.relative_to(repo.resolve())
    except ValueError:
        return None
    return candidate


def evidence_references(
    repo: Path,
    matrix_path: Path,
    row: dict[str, str],
    mechanism: dict[str, object] | None,
) -> list[str]:
    refs: set[str] = set()
    if isinstance(mechanism, dict):
        values = mechanism.get("bbb_evidence")
        if isinstance(values, list):
            refs.update(value for value in values if isinstance(value, str))
    for match in CODE_REF_RE.finditer(row["evidence"]):
        reference = match.group("path")
        if match.group("symbol"):
            reference += f"#{match.group('symbol')}"
        refs.add(reference)
    for raw in LINK_RE.findall(row["evidence"]):
        candidate = resolve_markdown_link(repo, matrix_path, raw)
        if candidate is not None and candidate.is_file():
            refs.add(candidate.relative_to(repo).as_posix())
    return sorted(refs)


def nested_code_references(value: object) -> Iterable[str]:
    if isinstance(value, str):
        for match in CODE_REF_RE.finditer(value):
            reference = match.group("path")
            if match.group("symbol"):
                reference += f"#{match.group('symbol')}"
            yield reference
    elif isinstance(value, dict):
        for child in value.values():
            yield from nested_code_references(child)
    elif isinstance(value, list):
        for child in value:
            yield from nested_code_references(child)


def nested_document_references(path: Path) -> Iterable[str]:
    text = path.read_text(encoding="utf-8")
    yield from nested_code_references(text)


@lru_cache(maxsize=None)
def declared_document_mechanism_ids(path: Path) -> frozenset[str]:
    if path.suffix != ".md":
        return frozenset()
    return frozenset(
        MECHANISM_ID_DECLARATION_RE.findall(path.read_text(encoding="utf-8"))
    )


def reference_path(reference: str) -> str:
    return reference.split("#", 1)[0]


def reference_symbol(reference: str) -> str | None:
    _path, separator, symbol = reference.partition("#")
    return symbol if separator and symbol else None


@lru_cache(maxsize=None)
def test_symbols(path: Path) -> dict[str, bool]:
    if path.suffix not in {".rs", ".py"}:
        return {}
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".py":
        return {name: False for name in PYTHON_TEST_RE.findall(text)}
    result: dict[str, bool] = {}
    for match in RUST_TEST_RE.finditer(text):
        attrs = match.group("attrs")
        if RUST_EXECUTABLE_TEST_ATTR_RE.search(attrs) is None:
            continue
        result[match.group("name")] = "#[ignore" in attrs
    return result


def focused_test_references(
    repo: Path, initial: Iterable[str], mechanism_id: str
) -> list[dict[str, object]]:
    queue = list(initial)
    visited: set[str] = set()
    candidates: set[str] = set()
    while queue:
        reference = queue.pop()
        if reference in visited:
            continue
        visited.add(reference)
        relative = reference_path(reference)
        path = repo / relative
        if not path.is_file():
            continue
        symbol = reference_symbol(reference)
        symbols = test_symbols(path)
        if symbol is not None:
            leaf = symbol.rsplit("::", 1)[-1]
            if leaf in symbols:
                candidates.add(f"{relative}#{leaf}")
        else:
            if path.suffix == ".json":
                try:
                    queue.extend(nested_code_references(json.loads(path.read_text(encoding="utf-8"))))
                except json.JSONDecodeError:
                    pass
            elif path.suffix == ".md":
                declared_ids = declared_document_mechanism_ids(path)
                if not declared_ids or mechanism_id in declared_ids:
                    queue.extend(nested_document_references(path))
            elif path.name.startswith("test_"):
                candidates.update(f"{relative}#{name}" for name in symbols)
    tests: list[dict[str, object]] = []
    for reference in sorted(candidates):
        relative, symbol = reference.split("#", 1)
        path = repo / relative
        ignored = test_symbols(path).get(symbol, False)
        if relative.startswith("crates/"):
            crate = Path(relative).parts[1]
            command = f"scripts/cargo-dev.sh test -p {crate} {symbol}"
        else:
            command = "scripts/dev test python --suite all"
        tests.append(
            {
                "command": command,
                "ignored": ignored,
                "reference": reference,
                "source_sha256": sha256_file(path),
                "result": "ignored" if ignored else "passed-via-full-gate",
            }
        )
    return tests


def gate_receipt_sha256(receipt: str) -> str:
    """Hash one UTF-8/LF framed gate receipt, not its append-only container."""
    return sha256_bytes(f"{receipt}\n".encode("utf-8"))


def gate_receipt(repo: Path, gate_log: Path, commit: str) -> dict[str, object]:
    if not gate_log.is_file():
        return {
            "artifact_id": "full-gate",
            "bbb_commit": commit,
            "digest_kind": GATE_RECEIPT_DIGEST_KIND,
            "exists": False,
            "invalid_reasons": ["gate log is absent"],
            "kind": "gate",
            "path": gate_log.relative_to(repo).as_posix()
            if gate_log.is_relative_to(repo)
            else str(gate_log),
            "sha256": None,
            "valid": False,
        }
    selected = None
    for line in gate_log.read_text(encoding="utf-8").splitlines():
        cells = line.split("\t")
        if len(cells) >= 4 and cells[1] == commit and cells[2] == "clean":
            selected = line
    reasons = [] if selected else [f"no clean full-gate receipt for {commit}"]
    return {
        "artifact_id": "full-gate",
        "bbb_commit": commit,
        "digest_kind": GATE_RECEIPT_DIGEST_KIND,
        "exists": True,
        "invalid_reasons": reasons,
        "kind": "gate",
        "path": gate_log.relative_to(repo).as_posix()
        if gate_log.is_relative_to(repo)
        else str(gate_log),
        "receipt": selected,
        "sha256": gate_receipt_sha256(selected) if selected is not None else None,
        "valid": not reasons,
    }


def current_gate_receipt_issues(
    artifact: dict[str, object], path: Path, artifact_id: str
) -> list[str]:
    """Validate the immutable selected line inside an append-only gate log."""
    issues: list[str] = []
    receipt = artifact.get("receipt")
    if artifact.get("digest_kind") != GATE_RECEIPT_DIGEST_KIND:
        issues.append(f"external gate receipt digest kind is stale: {artifact_id}")
    if not isinstance(receipt, str):
        issues.append(f"external gate receipt is absent: {artifact_id}")
        return issues
    if artifact.get("sha256") != gate_receipt_sha256(receipt):
        issues.append(f"external gate receipt hash is stale: {artifact_id}")
    cells = receipt.split("\t")
    if (
        len(cells) < 4
        or cells[1] != artifact.get("bbb_commit")
        or cells[2] != "clean"
    ):
        issues.append(f"external gate receipt identity is stale: {artifact_id}")
    if receipt not in path.read_text(encoding="utf-8").splitlines():
        issues.append(f"external gate receipt is absent: {artifact_id}")
    return issues


def artifact_commit(kind: str, value: object, path: Path) -> str | None:
    if not isinstance(value, dict):
        return None
    for key in ("head", "bbb_commit", "commit"):
        candidate = value.get(key)
        if isinstance(candidate, str) and re.fullmatch(r"[0-9a-f]{40}", candidate):
            return candidate
    if kind == "workload":
        review = value.get("visual_review")
        if isinstance(review, dict) and isinstance(review.get("note"), str):
            match = re.search(r"\b([0-9a-f]{7,40})\b", review["note"])
            if match:
                return match.group(1)
    if kind == "lifecycle":
        stamp = path.parent / "last-lifecycle-soak"
        if stamp.is_file():
            candidate = stamp.read_text(encoding="utf-8").strip()
            if re.fullmatch(r"[0-9a-f]{40}", candidate):
                return candidate
    return None


def artifact_binary_provenance(
    repo: Path, value: dict[str, object]
) -> tuple[dict[str, object] | None, list[str]]:
    """Validate the capture/evidence commit bridge carried by visual receipts."""
    provenance = value.get("bbb_provenance")
    if not isinstance(provenance, dict):
        return None, ["artifact lacks machine-readable BBB binary provenance"]
    capture_commit = provenance.get("capture_commit")
    evidence_commit = provenance.get("evidence_commit")
    binary_sha256 = provenance.get("binary_sha256")
    reasons: list[str] = []
    for label, commit in (
        ("capture", capture_commit),
        ("evidence", evidence_commit),
    ):
        if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
            reasons.append(f"BBB {label} commit is not exact")
    binary_identical = provenance.get("binary_identical") is True
    runtime_image_identical = provenance.get("runtime_image_identical") is True
    identity_kind = provenance.get("identity_kind")
    runtime_image_sha256 = provenance.get("runtime_image_sha256")
    if not binary_identical and not runtime_image_identical:
        reasons.append("BBB capture/evidence execution images are not declared identical")
    if not binary_identical:
        if identity_kind != ELF_LOAD_IMAGE_ALGORITHM:
            reasons.append("BBB non-byte-identical provenance lacks ELF load-image identity")
        if not isinstance(runtime_image_sha256, str) or re.fullmatch(
            r"[0-9a-f]{64}", runtime_image_sha256
        ) is None:
            reasons.append("BBB runtime image provenance has no valid SHA-256")
    if not isinstance(binary_sha256, str) or re.fullmatch(
        r"[0-9a-f]{64}", binary_sha256
    ) is None:
        reasons.append("BBB binary provenance has no valid SHA-256")

    normalized_binaries: dict[str, dict[str, object]] = {}
    for label in ("capture", "evidence"):
        binary = provenance.get(f"{label}_binary")
        if not isinstance(binary, dict):
            reasons.append(f"BBB provenance lacks {label} binary")
            continue
        path_value = binary.get("path")
        declared = binary.get("sha256")
        if not isinstance(path_value, str) or not Path(path_value).is_absolute():
            reasons.append(f"BBB {label} binary path is not absolute")
            continue
        path = Path(path_value)
        if not path.is_file():
            reasons.append(f"BBB {label} binary is absent: {path}")
            continue
        actual = sha256_file(path)
        if declared != actual:
            reasons.append(f"BBB {label} binary SHA-256 is stale")
        if binary_sha256 != actual:
            if label == "capture":
                reasons.append("BBB capture binary differs from captured binary identity")
            elif binary_identical:
                reasons.append("BBB evidence binary differs from captured binary identity")
        normalized_binaries[label] = {
            "path": str(path),
            "sha256": actual,
            "bytes": path.stat().st_size,
        }
        if not binary_identical and identity_kind == ELF_LOAD_IMAGE_ALGORITHM:
            try:
                runtime_identity = elf_load_image_identity(path)
            except (OSError, ExecutableIdentityError) as error:
                reasons.append(f"BBB {label} runtime image is invalid: {error}")
            else:
                actual_runtime_sha256 = runtime_identity["sha256"]
                if actual_runtime_sha256 != runtime_image_sha256:
                    reasons.append(f"BBB {label} runtime image identity is stale")
                normalized_binaries[label]["runtime_image_sha256"] = (
                    actual_runtime_sha256
                )
    if (
        isinstance(capture_commit, str)
        and isinstance(evidence_commit, str)
        and capture_commit != evidence_commit
    ):
        ancestor = subprocess.run(
            ["git", "merge-base", "--is-ancestor", capture_commit, evidence_commit],
            cwd=repo,
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if ancestor.returncode != 0:
            reasons.append(
                "BBB capture commit is not an ancestor of the evidence commit"
            )
    normalized: dict[str, object] = {
        "capture_commit": capture_commit,
        "evidence_commit": evidence_commit,
        "binary_sha256": binary_sha256,
        "binary_identical": binary_identical,
        "runtime_image_identical": runtime_image_identical,
        "identity_kind": identity_kind,
        "runtime_image_sha256": runtime_image_sha256,
        "capture_binary": normalized_binaries.get("capture"),
        "evidence_binary": normalized_binaries.get("evidence"),
    }
    return normalized, reasons


def artifact_scenario_ids(kind: str, value: dict[str, object]) -> list[str]:
    """Extract stable experiment names without interpreting their outcome."""
    candidates: object = None
    if kind == "workload":
        candidates = value.get("screenshot_pairs")
    elif kind == "draw":
        candidates = value.get("captures")
    if not isinstance(candidates, list):
        return []
    scenario_ids = {
        item.get("capture_segment")
        for item in candidates
        if isinstance(item, dict) and isinstance(item.get("capture_segment"), str)
    }
    return sorted(scenario_ids)


def nested_hash_inputs(value: object, prefix: str = "") -> list[dict[str, str]]:
    """Collect explicit capture/tool/input identities without hashing prose."""
    values: list[dict[str, str]] = []
    if isinstance(value, dict):
        for key, child in sorted(value.items()):
            child_prefix = f"{prefix}.{key}" if prefix else str(key)
            if isinstance(child, str) and (
                key.endswith(("_sha256", "_hash"))
                or key in {"sha256", "manifest_sha256"}
            ):
                values.append({"field": child_prefix, "value": child})
            elif key not in {"runs", "captures", "screenshot_pairs", "iterations"}:
                values.extend(nested_hash_inputs(child, child_prefix))
    elif isinstance(value, list) and len(value) <= 32:
        for index, child in enumerate(value):
            values.extend(nested_hash_inputs(child, f"{prefix}[{index}]"))
    return values


def git_blob_sha256(repo: Path, commit: str | None, relative: str) -> str | None:
    if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
        return None
    result = subprocess.run(
        ["git", "show", f"{commit}:{relative}"],
        cwd=repo,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    return sha256_bytes(result.stdout) if result.returncode == 0 else None


def capture_path_identities(
    repo: Path, commit: str | None, paths: Iterable[str]
) -> list[dict[str, str | None]]:
    return [
        {
            "capture_sha256": git_blob_sha256(repo, commit, relative),
            "current_sha256": sha256_file(repo / relative),
            "path": relative,
        }
        for relative in paths
        if (repo / relative).is_file()
    ]


def artifact_binary_identity(value: dict[str, object]) -> dict[str, object] | None:
    provenance = value.get("bbb_provenance")
    if isinstance(provenance, dict):
        identity = {
            key: provenance[key]
            for key in (
                "binary_identical",
                "binary_sha256",
                "capture_commit",
                "evidence_commit",
                "identity_kind",
                "runtime_image_identical",
                "runtime_image_sha256",
                "scope",
            )
            if key in provenance
        }
        if identity:
            return identity
    release = value.get("release")
    if isinstance(release, dict) and isinstance(release.get("sha256"), str):
        return {
            "binary": release.get("binary"),
            "binary_sha256": release["sha256"],
            "identity_kind": "file-sha256",
        }
    if isinstance(value.get("client_binary_sha256"), str):
        return {
            "binary": value.get("client_binary"),
            "binary_sha256": value["client_binary_sha256"],
            "identity_kind": "file-sha256",
        }
    return None


def capture_contract_identity(
    repo: Path,
    kind: str,
    value: dict[str, object],
    evidence_class: str,
    capture_commit: str | None,
) -> dict[str, object]:
    selected = {
        key: value[key]
        for key in (
            "schema",
            "schema_version",
            "contracts",
            "counter_scope",
            "formal_timing_sample",
            "manifest_sha256",
            "metric_scope",
            "prerequisites",
            "row",
            "scope",
            "server_camera_log_sha256",
        )
        if key in value
    }
    identity = {
        "binary_load_identity": artifact_binary_identity(value),
        "evidence_class": evidence_class,
        "hash_inputs": nested_hash_inputs(value),
        "inputs": capture_path_identities(
            repo, capture_commit, CAPTURE_INPUT_PATHS.get(kind, ())
        ),
        "kind": kind,
        "selected_contract": selected,
        "tools": capture_path_identities(
            repo, capture_commit, CAPTURE_TOOL_PATHS.get(kind, ())
        ),
    }
    identity["sha256"] = canonical_sha256(identity)
    return identity


def artifact_evidence_class(kind: str, value: dict[str, object]) -> str:
    explicit = value.get("evidence_class")
    if explicit in {"formal", "diagnostic"}:
        return str(explicit)
    mode = str(value.get("mode", "")).lower()
    if "smoke" in mode or "diagnostic" in mode:
        return "diagnostic"
    if kind == "performance" and value.get("formal_comparison_complete") is not True:
        return "diagnostic"
    if kind == "audio" and (
        value.get("formal_valid") is not True or value.get("driver") == "null"
    ):
        return "diagnostic"
    return "formal"


def performance_prerequisite_reasons(
    repo: Path, value: dict[str, object], evidence_commit: str
) -> list[str]:
    prerequisites = value.get("prerequisites")
    receipt = (
        prerequisites.get("consistency_receipt")
        if isinstance(prerequisites, dict)
        else None
    )
    if not isinstance(receipt, dict):
        return ["formal ABBA lacks a pre-capture consistency receipt"]
    path = receipt.get("path")
    report_commit = receipt.get("report_commit")
    digest = receipt.get("sha256")
    if (
        path != REPORT_PATH.as_posix()
        or not isinstance(report_commit, str)
        or re.fullmatch(r"[0-9a-f]{40}", report_commit) is None
        or not isinstance(digest, str)
    ):
        return ["formal ABBA consistency receipt identity is malformed"]
    blob = subprocess.run(
        ["git", "show", f"{report_commit}:{path}"],
        cwd=repo,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if blob.returncode != 0 or sha256_bytes(blob.stdout) != digest:
        return ["formal ABBA consistency receipt is not sealed by its commit"]
    try:
        report = json.loads(blob.stdout)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return ["formal ABBA consistency receipt JSON is invalid"]
    workflow = report.get("workflow") if isinstance(report, dict) else None
    if (
        not isinstance(workflow, dict)
        or workflow.get("stage") != "evidence-sweep"
        or workflow.get("consistency_gate_passed") is not True
        or report.get("bbb_evidence_commit") != evidence_commit
        or receipt.get("bbb_evidence_commit") != evidence_commit
        or workflow.get("semantic_contract_sha256")
        != receipt.get("semantic_contract_sha256")
        or receipt.get("stage") != "evidence-sweep"
    ):
        return ["formal ABBA consistency receipt did not pass for the evidence commit"]
    ancestry = subprocess.run(
        ["git", "merge-base", "--is-ancestor", evidence_commit, report_commit],
        cwd=repo,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    changed = subprocess.run(
        ["git", "diff", "--name-only", evidence_commit, report_commit, "--"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    changed_paths = set(changed.stdout.splitlines()) if changed.returncode == 0 else set()
    if ancestry.returncode != 0 or changed.returncode != 0 or any(
        not is_closure_metadata_path(path) for path in changed_paths
    ):
        return ["formal ABBA consistency receipt includes a functional code delta"]
    closure_paths = changed_paths - {REPORT_PATH.as_posix()}
    if closure_paths:
        baseline_blob = subprocess.run(
            ["git", "show", f"{evidence_commit}:{REPORT_PATH.as_posix()}"],
            cwd=repo,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        try:
            baseline_report = json.loads(baseline_blob.stdout)
        except (UnicodeDecodeError, json.JSONDecodeError):
            baseline_report = None
        baseline_workflow = (
            baseline_report.get("workflow")
            if isinstance(baseline_report, dict)
            else None
        )
        impact = report.get("change_impact")
        if (
            baseline_blob.returncode != 0
            or not isinstance(baseline_workflow, dict)
            or baseline_workflow.get("semantic_contract_sha256")
            != workflow.get("semantic_contract_sha256")
            or not isinstance(impact, dict)
            or impact.get("functional_changed_paths") != []
            or impact.get("functional_affected_mechanism_ids") != []
        ):
            return [
                "formal ABBA consistency receipt closure metadata changed its semantic contract"
            ]
    git_dir_result = subprocess.run(
        ["git", "rev-parse", "--absolute-git-dir"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if git_dir_result.returncode != 0:
        return ["formal ABBA consistency receipt gate metadata is unavailable"]
    gate_log = Path(git_dir_result.stdout.strip()) / "bbb/gate-log.tsv"
    gate_line = None
    if gate_log.is_file():
        for line in gate_log.read_text(encoding="utf-8").splitlines():
            cells = line.split("\t")
            if len(cells) >= 4 and cells[1] == report_commit and cells[2] == "clean":
                gate_line = line
    if gate_line is None or receipt.get(
        "full_gate_receipt_sha256"
    ) != gate_receipt_sha256(gate_line):
        return ["formal ABBA consistency receipt commit was not clean full-gated"]
    return []


def external_artifact(
    *,
    repo: Path,
    kind: str,
    artifact_id: str,
    path: Path | None,
    evidence_commit: str,
) -> dict[str, object]:
    if path is None:
        return {
            "artifact_id": artifact_id,
            "bbb_commit": None,
            "exists": False,
            "evidence_class": "absent",
            "capture_contract": None,
            "invalid_reasons": [f"{kind} artifact was not supplied"],
            "kind": kind,
            "path": None,
            "scenario_ids": [],
            "sha256": None,
            "valid": False,
        }
    if not path.is_file():
        return {
            "artifact_id": artifact_id,
            "bbb_commit": None,
            "exists": False,
            "evidence_class": "absent",
            "capture_contract": None,
            "invalid_reasons": [f"artifact does not exist: {path}"],
            "kind": kind,
            "path": str(path),
            "scenario_ids": [],
            "sha256": None,
            "valid": False,
        }
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        return {
            "artifact_id": artifact_id,
            "bbb_commit": None,
            "exists": True,
            "evidence_class": "invalid",
            "capture_contract": None,
            "invalid_reasons": [f"invalid JSON artifact: {error}"],
            "kind": kind,
            "path": str(path),
            "scenario_ids": [],
            "sha256": sha256_file(path),
            "valid": False,
        }
    if not isinstance(value, dict):
        return {
            "artifact_id": artifact_id,
            "bbb_commit": None,
            "exists": True,
            "evidence_class": "invalid",
            "capture_contract": None,
            "invalid_reasons": ["artifact JSON root is not an object"],
            "kind": kind,
            "path": str(path),
            "scenario_ids": [],
            "sha256": sha256_file(path),
            "valid": False,
        }
    commit = artifact_commit(kind, value, path)
    reasons: list[str] = []
    evidence_class = artifact_evidence_class(kind, value)
    raw_provenance = value.get("bbb_provenance")
    capture_tool_commit = (
        raw_provenance.get("capture_commit")
        if isinstance(raw_provenance, dict)
        else None
    )
    if not isinstance(capture_tool_commit, str):
        capture_tool_commit = commit
    capture_contract = capture_contract_identity(
        repo, kind, value, evidence_class, capture_tool_commit
    )
    changed_capture_paths = {
        role: [
            str(item["path"])
            for item in capture_contract[role]
            if item.get("capture_sha256") != item.get("current_sha256")
        ]
        for role in ("tools", "inputs")
    }
    if changed_capture_paths["tools"]:
        reasons.append(
            "capture tool changed after the artifact: "
            + ", ".join(changed_capture_paths["tools"])
        )
    if changed_capture_paths["inputs"]:
        reasons.append(
            "capture input changed after the artifact: "
            + ", ".join(changed_capture_paths["inputs"])
        )
    provenance = None
    if kind in {"performance", "workload", "draw"}:
        provenance, provenance_reasons = artifact_binary_provenance(repo, value)
        reasons.extend(provenance_reasons)
    if kind == "performance" and evidence_class == "formal":
        reasons.extend(performance_prerequisite_reasons(repo, value, evidence_commit))
    status = value.get("status") if isinstance(value, dict) else None
    if kind == "performance" and value.get("formal_comparison_complete") is not True:
        reasons.append("formal comparison is incomplete")
    elif kind == "workload" and status != "matched":
        reasons.append(f"workload status is {status!r}, not 'matched'")
    elif kind == "draw" and status != "validated":
        reasons.append(f"draw status is {status!r}, not 'validated'")
    elif kind == "platform" and status != "passed":
        reasons.append(f"platform status is {status!r}, not 'passed'")
    elif kind == "audio" and value.get("formal_valid") is not True:
        reasons.append("formal host-device audio receipt is invalid")
    if kind == "audio":
        reasons.extend(formal_audio_receipt_reasons(value))
        reasons.extend(audio_receipt_file_reasons(value))
    if evidence_class != "formal":
        reasons.append("diagnostic smoke cannot satisfy a formal receipt")
    if (
        evidence_class == "formal"
        and kind in FORMAL_BINARY_IDENTITY_KINDS
        and capture_contract["binary_load_identity"] is None
    ):
        reasons.append("formal receipt lacks BBB binary/load-image identity")
    if commit is None:
        reasons.append("artifact does not identify the BBB commit")
    if isinstance(provenance, dict) and provenance.get("evidence_commit") != commit:
        reasons.append("artifact BBB commit differs from binary provenance")
    scoped_reuse = None
    if commit is not None and not evidence_commit.startswith(commit) and commit != evidence_commit:
        if kind in CAPTURE_TOOL_PATHS:
            ancestry = subprocess.run(
                ["git", "merge-base", "--is-ancestor", commit, evidence_commit],
                cwd=repo,
                check=False,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            changed = subprocess.run(
                ["git", "diff", "--name-only", commit, evidence_commit, "--"],
                cwd=repo,
                check=False,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
            )
            changed_paths = (
                sorted(set(changed.stdout.splitlines()))
                if changed.returncode == 0
                else []
            )
            disallowed_paths = [
                path
                for path in changed_paths
                if not is_closure_metadata_path(path)
                and path not in SCOPED_FORMAL_RECEIPT_REUSE_PATHS
            ]
            if (
                ancestry.returncode == 0
                and changed.returncode == 0
                and not disallowed_paths
                and not changed_capture_paths["tools"]
                and not changed_capture_paths["inputs"]
            ):
                scoped_reuse = {
                    "artifact_commit": commit,
                    "changed_paths": changed_paths,
                    "evidence_commit": evidence_commit,
                    "rule": "closure-or-acceptance-metadata-only",
                }
            else:
                reasons.append(
                    f"artifact BBB commit {commit} differs from evidence commit {evidence_commit}"
                )
        else:
            reasons.append(
                f"artifact BBB commit {commit} differs from evidence commit {evidence_commit}"
            )
    return {
        "artifact_id": artifact_id,
        "bbb_commit": commit,
        "capture_contract": capture_contract,
        "evidence_class": evidence_class,
        "exists": True,
        "invalid_reasons": reasons,
        "kind": kind,
        "path": str(path),
        "bbb_provenance": provenance,
        "scenario_ids": artifact_scenario_ids(kind, value),
        "scoped_reuse": scoped_reuse,
        "sha256": sha256_file(path),
        "valid": not reasons,
    }


def reuse_external_artifact_projection(
    *,
    repo: Path,
    kind: str,
    artifact_id: str,
    prior: object,
    evidence_commit: str,
    source: dict[str, str],
) -> dict[str, object]:
    """Reuse a committed formal projection without claiming its file still exists."""

    if (
        not isinstance(prior, dict)
        or prior.get("artifact_id") != artifact_id
        or prior.get("kind") != kind
        or prior.get("valid") is not True
        or prior.get("evidence_class") != "formal"
    ):
        return external_artifact(
            repo=repo,
            kind=kind,
            artifact_id=artifact_id,
            path=None,
            evidence_commit=evidence_commit,
        )
    artifact = json.loads(json.dumps(prior))
    contract = artifact.get("capture_contract")
    reasons: list[str] = []
    changed_capture_paths: dict[str, list[str]] = {"tools": [], "inputs": []}
    if not isinstance(contract, dict):
        reasons.append("reused formal artifact lacks a capture contract")
    else:
        for role in ("tools", "inputs"):
            identities = contract.get(role)
            if not isinstance(identities, list):
                reasons.append(f"reused formal artifact has malformed capture {role}")
                continue
            for identity in identities:
                if not isinstance(identity, dict) or not isinstance(
                    identity.get("path"), str
                ):
                    reasons.append(f"reused formal artifact has malformed capture {role}")
                    continue
                path = repo / identity["path"]
                identity["current_sha256"] = (
                    sha256_file(path) if path.is_file() else None
                )
                if identity.get("capture_sha256") != identity.get("current_sha256"):
                    changed_capture_paths[role].append(str(identity["path"]))
        contract["sha256"] = canonical_sha256(
            {key: value for key, value in contract.items() if key != "sha256"}
        )
    if changed_capture_paths["tools"]:
        reasons.append(
            "capture tool changed after the artifact: "
            + ", ".join(changed_capture_paths["tools"])
        )
    if changed_capture_paths["inputs"]:
        reasons.append(
            "capture input changed after the artifact: "
            + ", ".join(changed_capture_paths["inputs"])
        )

    commit = artifact.get("bbb_commit")
    scoped_reuse = None
    if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
        reasons.append("reused artifact does not identify the BBB commit")
    elif commit != evidence_commit:
        ancestry = subprocess.run(
            ["git", "merge-base", "--is-ancestor", commit, evidence_commit],
            cwd=repo,
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        changed = subprocess.run(
            ["git", "diff", "--name-only", commit, evidence_commit, "--"],
            cwd=repo,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        changed_paths = (
            sorted(set(changed.stdout.splitlines()))
            if changed.returncode == 0
            else []
        )
        disallowed_paths = [
            path
            for path in changed_paths
            if not is_closure_metadata_path(path)
            and path not in SCOPED_FORMAL_RECEIPT_REUSE_PATHS
        ]
        if (
            ancestry.returncode == 0
            and changed.returncode == 0
            and not disallowed_paths
            and not changed_capture_paths["tools"]
            and not changed_capture_paths["inputs"]
        ):
            scoped_reuse = {
                "artifact_commit": commit,
                "changed_paths": changed_paths,
                "evidence_commit": evidence_commit,
                "rule": "closure-or-acceptance-metadata-only",
            }
        else:
            reasons.append(
                f"artifact BBB commit {commit} differs from evidence commit {evidence_commit}"
            )

    artifact["capture_contract"] = contract
    artifact["invalid_reasons"] = reasons
    artifact_path_value = artifact.get("path")
    artifact_path = (
        Path(artifact_path_value)
        if isinstance(artifact_path_value, str)
        else None
    )
    if artifact_path is not None and not artifact_path.is_absolute():
        artifact_path = repo / artifact_path
    file_present = artifact_path is not None and artifact_path.is_file()
    file_sha256 = sha256_file(artifact_path) if file_present else None
    if file_sha256 is not None and file_sha256 != artifact.get("sha256"):
        reasons.append("reused projection artifact file hash is stale")
    artifact["projection_reuse"] = {
        **source,
        "artifact_id": artifact_id,
        "artifact_file_present_at_generation": file_present,
        "artifact_file_sha256_at_generation": file_sha256,
        "rule": "committed-projection-only; final-requires-artifact",
    }
    artifact["scoped_reuse"] = scoped_reuse
    artifact["valid"] = not reasons
    return artifact


def projection_reuse_reasons(
    repo: Path, artifact: dict[str, object], evidence_commit: str
) -> list[str]:
    """Verify a reused projection against its exact committed report artifact."""

    metadata = artifact.get("projection_reuse")
    if metadata is None:
        return []
    if not isinstance(metadata, dict):
        return ["external artifact projection reuse metadata is malformed"]
    required = {
        "artifact_id",
        "artifact_file_present_at_generation",
        "artifact_file_sha256_at_generation",
        "rule",
        "source_report_blob_sha1",
        "source_report_commit",
        "source_report_path",
        "source_report_sha256",
    }
    if set(metadata) != required or metadata.get("rule") != (
        "committed-projection-only; final-requires-artifact"
    ):
        return ["external artifact projection reuse contract is malformed"]
    source_commit = metadata.get("source_report_commit")
    blob_sha1 = metadata.get("source_report_blob_sha1")
    report_sha256 = metadata.get("source_report_sha256")
    if (
        not isinstance(source_commit, str)
        or re.fullmatch(r"[0-9a-f]{40}", source_commit) is None
        or not isinstance(blob_sha1, str)
        or re.fullmatch(r"[0-9a-f]{40}", blob_sha1) is None
        or not isinstance(report_sha256, str)
        or re.fullmatch(r"[0-9a-f]{64}", report_sha256) is None
        or metadata.get("source_report_path") != REPORT_PATH.as_posix()
        or metadata.get("artifact_id") != artifact.get("artifact_id")
        or not isinstance(metadata.get("artifact_file_present_at_generation"), bool)
        or (
            metadata.get("artifact_file_sha256_at_generation") is not None
            and (
                not isinstance(
                    metadata.get("artifact_file_sha256_at_generation"), str
                )
                or re.fullmatch(
                    r"[0-9a-f]{64}",
                    str(metadata.get("artifact_file_sha256_at_generation")),
                )
                is None
            )
        )
        or (
            metadata.get("artifact_file_present_at_generation")
            is (metadata.get("artifact_file_sha256_at_generation") is None)
        )
    ):
        return ["external artifact projection reuse identity is malformed"]
    ancestry = subprocess.run(
        ["git", "merge-base", "--is-ancestor", source_commit, evidence_commit],
        cwd=repo,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    show = subprocess.run(
        ["git", "show", f"{source_commit}:{REPORT_PATH.as_posix()}"],
        cwd=repo,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    blob = subprocess.run(
        ["git", "rev-parse", f"{source_commit}:{REPORT_PATH.as_posix()}"],
        cwd=repo,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if (
        ancestry.returncode != 0
        or show.returncode != 0
        or blob.returncode != 0
        or blob.stdout.strip() != blob_sha1
        or hashlib.sha256(show.stdout).hexdigest() != report_sha256
    ):
        return ["external artifact projection source report is stale"]
    try:
        source_report = json.loads(show.stdout.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return ["external artifact projection source report is malformed"]
    source_values = source_report.get("artifacts") if isinstance(source_report, dict) else None
    if not isinstance(source_values, list):
        return ["external artifact projection source report lacks artifacts"]
    source_artifact = next(
        (
            item
            for item in source_values
            if isinstance(item, dict)
            and item.get("artifact_id") == artifact.get("artifact_id")
        ),
        None,
    )
    if (
        not isinstance(source_artifact, dict)
        or source_artifact.get("valid") is not True
        or source_artifact.get("evidence_class") != "formal"
    ):
        return ["external artifact projection source was not a valid formal receipt"]
    immutable_keys = (
        "artifact_id",
        "bbb_commit",
        "bbb_provenance",
        "evidence_class",
        "exists",
        "kind",
        "path",
        "scenario_ids",
        "sha256",
    )
    if any(artifact.get(key) != source_artifact.get(key) for key in immutable_keys):
        return ["external artifact projection diverges from its committed source"]

    def capture_basis(value: object) -> object:
        if not isinstance(value, dict):
            return value
        result = json.loads(json.dumps(value))
        result.pop("sha256", None)
        for role in ("tools", "inputs"):
            identities = result.get(role)
            if isinstance(identities, list):
                for identity in identities:
                    if isinstance(identity, dict):
                        identity.pop("current_sha256", None)
        return result

    if capture_basis(artifact.get("capture_contract")) != capture_basis(
        source_artifact.get("capture_contract")
    ):
        return ["external artifact projection capture contract was rewritten"]
    return []


def repo_artifact_generator_inputs(
    repo: Path, path: Path, artifact_id: str
) -> tuple[list[dict[str, str]] | None, list[str]]:
    if path.suffix != ".json" or not path.is_file():
        return None, []
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None, []
    if not isinstance(value, dict) or "generator_inputs" not in value:
        return None, []
    raw_inputs = value.get("generator_inputs")
    if not isinstance(raw_inputs, list):
        return [], [f"repo artifact generator inputs are malformed: {artifact_id}"]
    inputs: list[dict[str, str]] = []
    reasons: list[str] = []
    seen_paths: set[str] = set()
    for item in raw_inputs:
        relative = item.get("path") if isinstance(item, dict) else None
        digest = item.get("sha256") if isinstance(item, dict) else None
        if (
            not isinstance(relative, str)
            or not relative
            or Path(relative).is_absolute()
            or ".." in Path(relative).parts
            or not isinstance(digest, str)
            or re.fullmatch(r"[0-9a-f]{64}", digest) is None
        ):
            reasons.append(f"repo artifact generator input is malformed: {artifact_id}")
            continue
        if relative in seen_paths:
            reasons.append(
                f"repo artifact generator input is duplicated: {artifact_id}: {relative}"
            )
            continue
        seen_paths.add(relative)
        inputs.append({"path": relative, "sha256": digest})
        candidate = repo / relative
        if not candidate.is_file() or sha256_file(candidate) != digest:
            reasons.append(
                f"repo artifact generator input is stale: {artifact_id}: {relative}"
            )
    return sorted(inputs, key=lambda item: item["path"]), sorted(set(reasons))


def repo_artifact(repo: Path, artifact_id: str, relative: str) -> dict[str, object]:
    path = repo / relative
    generator_inputs, generator_reasons = repo_artifact_generator_inputs(
        repo, path, artifact_id
    )
    exists = path.is_file()
    reasons = [] if exists else [f"missing repo evidence: {relative}"]
    reasons.extend(generator_reasons)
    artifact: dict[str, object] = {
        "artifact_id": artifact_id,
        "bbb_commit": None,
        "exists": exists,
        "invalid_reasons": reasons,
        "kind": "repo-evidence",
        "path": relative,
        "scenario_ids": [],
        "sha256": sha256_file(path) if exists else None,
        "valid": not reasons,
    }
    if generator_inputs is not None:
        artifact["generator_inputs"] = generator_inputs
    return artifact


def artifacts_with_current_repo_inputs(
    repo: Path, artifacts: Iterable[dict[str, object]]
) -> list[dict[str, object]]:
    values: list[dict[str, object]] = []
    for artifact in artifacts:
        if (
            artifact.get("kind") != "repo-evidence"
            or not isinstance(artifact.get("artifact_id"), str)
            or not isinstance(artifact.get("path"), str)
        ):
            values.append(artifact)
            continue
        refreshed = repo_artifact(
            repo, str(artifact["artifact_id"]), str(artifact["path"])
        )
        refreshed["mechanism_ids"] = artifact.get("mechanism_ids", [])
        values.append(refreshed)
    return values


def external_artifact_ids(row: dict[str, str]) -> list[str]:
    mechanism_id = row["mechanism_id"]
    classes = set(validation_classes(row))
    result: list[str] = []
    if "audio-host-device" in classes:
        result.append("audio-host-device")
    if "performance-abba" in classes:
        result.append("performance-abba")
    if "visual-workload" in classes:
        result.append("visual-workload")
    if "draw-evidence" in classes:
        result.append("draw-evidence")
    if classes & {"protocol-state-receipt", "resource-platform-receipt"}:
        result.append("linux-platform")
    if mechanism_id.startswith("lifecycle-") or mechanism_id == "resources-indexed-panorama":
        result.append("lifecycle-soak")
    return sorted(set(result))


def derived_open_reasons(
    *,
    status: str,
    owner_open_gaps: int,
    has_implementation_evidence: bool,
    focused_current: bool,
    external_ids: Iterable[str],
    artifacts: dict[str, dict[str, object]],
    workflow_stage: str,
) -> list[str]:
    if status not in OPEN_STATUSES:
        return []
    reasons: list[str] = []
    external_ids = list(external_ids)
    if owner_open_gaps or not has_implementation_evidence:
        reasons.append("implementation-gap")
    if not focused_current:
        reasons.append("focused-evidence-pending")
    if status == "deferred-external" or owner_open_gaps:
        reasons.append("external-blocker")
    if status != "deferred-external" and (
        not external_ids
        or any(not artifacts[artifact_id]["valid"] for artifact_id in external_ids)
        or workflow_stage_index(workflow_stage)
        < workflow_stage_index("evidence-sweep")
    ):
        reasons.append("formal-receipt-pending")
    if not reasons:
        reasons.append("formal-receipt-pending")
    return sorted(set(reasons))


def differential_kind(row: dict[str, str]) -> str:
    domain = row["domain"]
    if "protocol" in domain:
        return "packet-trace"
    if "render" in domain or row["mechanism_id"] in VISUAL_MECHANISMS:
        return "visual/draw"
    if "platform" in domain:
        return "platform-receipt"
    return "state-trace"


def build_report(
    *,
    repo: Path,
    source_root: Path,
    evidence_commit: str,
    gate_log: Path,
    workflow_stage: str = "implementation",
    performance_summary: Path | None = None,
    workload_summary: Path | None = None,
    draw_summary: Path | None = None,
    platform_report: Path | None = None,
    lifecycle_report: Path | None = None,
    audio_report: Path | None = None,
    reused_external_artifacts: dict[str, dict[str, object]] | None = None,
    projection_source: dict[str, str] | None = None,
) -> dict[str, object]:
    workflow_stage_index(workflow_stage)
    repo = repo.resolve()
    matrix_path = repo / MATRIX_PATH
    inventory_path = repo / INVENTORY_PATH
    ledger_path = repo / LEDGER_PATH
    rows = parse_matrix(matrix_path)
    inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
    if inventory.get("vanilla_version") != "26.1":
        raise AcceptanceError("client mechanism inventory is not Vanilla 26.1")
    mechanisms = inventory.get("mechanisms")
    owners_value = inventory.get("owners")
    source_files = inventory.get("source_files")
    if not isinstance(mechanisms, dict) or not isinstance(owners_value, list):
        raise AcceptanceError("malformed client mechanism inventory")
    if not isinstance(source_files, list):
        raise AcceptanceError("inventory lacks source hashes")
    inventory_hashes = {
        item["path"]: item["sha256"]
        for item in source_files
        if isinstance(item, dict)
        and isinstance(item.get("path"), str)
        and isinstance(item.get("sha256"), str)
    }
    corpus_source_sha256 = canonical_sha256(source_files)
    owners_by_mechanism: dict[str, list[dict[str, object]]] = defaultdict(list)
    for owner in owners_value:
        if isinstance(owner, dict) and isinstance(owner.get("mechanism_id"), str):
            owners_by_mechanism[owner["mechanism_id"]].append(owner)
    ledger_text = ledger_path.read_text(encoding="utf-8")
    ledger_by_id, ledger_ids = ledger_sections(ledger_text)
    class_sources = source_index(source_root)
    changed_paths = changed_repo_paths(repo, evidence_commit)
    changed_paths.discard(REPORT_PATH.as_posix())

    def external_or_reused(
        *, kind: str, artifact_id: str, path: Path | None
    ) -> dict[str, object]:
        if (
            path is None
            and reused_external_artifacts is not None
            and projection_source is not None
            and artifact_id in reused_external_artifacts
        ):
            return reuse_external_artifact_projection(
                repo=repo,
                kind=kind,
                artifact_id=artifact_id,
                prior=reused_external_artifacts[artifact_id],
                evidence_commit=evidence_commit,
                source=projection_source,
            )
        return external_artifact(
            repo=repo,
            kind=kind,
            artifact_id=artifact_id,
            path=path,
            evidence_commit=evidence_commit,
        )

    artifacts: dict[str, dict[str, object]] = {
        "parity-matrix": repo_artifact(repo, "parity-matrix", MATRIX_PATH.as_posix()),
        "client-owner-inventory": repo_artifact(
            repo, "client-owner-inventory", INVENTORY_PATH.as_posix()
        ),
        "feature-ledger": repo_artifact(repo, "feature-ledger", LEDGER_PATH.as_posix()),
        "full-gate": gate_receipt(repo, gate_log, evidence_commit),
        "performance-abba": external_or_reused(
            kind="performance",
            artifact_id="performance-abba",
            path=performance_summary,
        ),
        "visual-workload": external_or_reused(
            kind="workload",
            artifact_id="visual-workload",
            path=workload_summary,
        ),
        "draw-evidence": external_or_reused(
            kind="draw",
            artifact_id="draw-evidence",
            path=draw_summary,
        ),
        "linux-platform": external_or_reused(
            kind="platform",
            artifact_id="linux-platform",
            path=platform_report,
        ),
        "lifecycle-soak": external_or_reused(
            kind="lifecycle",
            artifact_id="lifecycle-soak",
            path=lifecycle_report,
        ),
        "audio-host-device": external_or_reused(
            kind="audio",
            artifact_id="audio-host-device",
            path=audio_report,
        ),
    }
    consistency_rows = [
        row
        for row in rows
        if not row["mechanism_id"].startswith("performance-")
        and set(validation_classes(row))
        & {
            "protocol-state-receipt",
            "resource-platform-receipt",
            "visual-workload",
            "draw-evidence",
        }
    ]
    consistency_gate_passed = artifacts["full-gate"]["valid"] and all(
        row["status"] in {"equivalent", "not-needed", "deferred-external"}
        and all(
            artifacts[artifact_id]["valid"]
            for artifact_id in external_artifact_ids(row)
            if artifact_id != "performance-abba"
        )
        for row in consistency_rows
    )
    performance_artifact = artifacts["performance-abba"]
    artifact_mechanisms: dict[str, set[str]] = defaultdict(set)
    scenario_mechanisms: dict[str, set[str]] = defaultdict(set)
    scenario_artifacts: dict[str, set[str]] = defaultdict(set)
    scenario_kinds: dict[str, set[str]] = defaultdict(set)
    scenario_statuses: dict[str, set[str]] = defaultdict(set)
    records: list[dict[str, object]] = []

    for row in rows:
        mechanism_id = row["mechanism_id"]
        owner_rows = owners_by_mechanism.get(mechanism_id, [])
        mechanism = mechanisms.get(mechanism_id)
        ledger_basis = ledger_by_id.get(mechanism_id, "")
        refs = evidence_references(
            repo, matrix_path, row, mechanism if isinstance(mechanism, dict) else None
        )
        owner_implementations: set[str] = set()
        owner_tests: set[str] = set()
        owner_open_gaps = 0
        for owner in owner_rows:
            bbb_evidence = owner.get("bbb_evidence")
            if not isinstance(bbb_evidence, dict):
                continue
            implementations = bbb_evidence.get("implementation")
            if isinstance(implementations, list):
                owner_implementations.update(
                    item for item in implementations if isinstance(item, str)
                )
            evidence_tests = bbb_evidence.get("tests")
            if isinstance(evidence_tests, list):
                owner_tests.update(
                    item for item in evidence_tests if isinstance(item, str)
                )
            if isinstance(bbb_evidence.get("open_gap"), dict):
                owner_open_gaps += 1
        evidence_basis_parts = [row["evidence"]]
        for reference in refs:
            path = repo / reference_path(reference)
            if path.is_file() and path.suffix == ".md":
                # The ledger is a shared routing document. Feeding the whole file to
                # one mechanism makes unrelated section edits silently replace that
                # mechanism's Vanilla source hashes. `ledger_basis` below is already
                # restricted to the sections that name this Mechanism ID.
                if path.resolve() == ledger_path.resolve():
                    continue
                evidence_basis_parts.append(path.read_text(encoding="utf-8"))
        source_hashes = mechanism_source_hashes(
            mechanism_id=mechanism_id,
            row=row,
            owners=owner_rows,
            inventory_hashes=inventory_hashes,
            class_sources=class_sources,
            source_root=source_root,
            ledger_basis=ledger_basis,
            evidence_basis="\n".join(evidence_basis_parts),
        )
        tests = focused_test_references(
            repo, [*refs, *owner_tests], row["mechanism_id"]
        )
        test_source_artifact_ids = sorted(
            {
                f"repo:{reference_path(str(test['reference']))}"
                for test in tests
                if isinstance(test.get("reference"), str)
            }
        )
        if len(tests) > 128:
            test_references = [str(test["reference"]) for test in tests]
            tests = [
                {
                    "command": "scripts/cargo-dev.sh gate",
                    "evidence_set_count": len(test_references),
                    "evidence_set_sha256": canonical_sha256(test_references),
                    "ignored": False,
                    "result": "passed-via-full-gate",
                    "source_artifact_ids": sorted(
                        test_source_artifact_ids
                    ),
                }
            ]
        focused_changed_paths = sorted(
            path
            for path in (
                artifact_id.removeprefix("repo:")
                for artifact_id in test_source_artifact_ids
            )
            if path in changed_paths and path not in VALIDATION_ONLY_PATHS
        )
        has_executable_tests = any(test.get("ignored") is not True for test in tests)
        focused_current = (
            has_executable_tests
            and artifacts["full-gate"]["valid"]
            and not focused_changed_paths
        )
        for test in tests:
            if test.get("ignored") is True:
                continue
            test["result"] = (
                "passed-via-full-gate"
                if focused_current
                else "focused-evidence-pending"
            )
        dependency_roles: dict[str, set[str]] = defaultdict(set)
        for reference in refs:
            dependency_roles[reference_path(reference)].add("matrix-evidence")
        for reference in owner_implementations:
            dependency_roles[reference_path(reference)].add("owner-implementation")
        for reference in owner_tests:
            dependency_roles[reference_path(reference)].add("owner-focused-test")
        for test in tests:
            reference = test.get("reference")
            if isinstance(reference, str):
                dependency_roles[reference_path(reference)].add("focused-test")
            for artifact_id in test.get("source_artifact_ids", []):
                if isinstance(artifact_id, str):
                    dependency_roles[artifact_id.removeprefix("repo:")].add(
                        "focused-test-set"
                    )
        dependency_closure = hashed_repo_paths(repo, dependency_roles)
        dependency_closure_sha256 = canonical_sha256(dependency_closure)
        focused_test_contract = [
            {
                key: test.get(key)
                for key in (
                    "command",
                    "evidence_set_count",
                    "evidence_set_sha256",
                    "ignored",
                    "reference",
                    "source_artifact_ids",
                    "source_sha256",
                )
                if key in test
            }
            for test in tests
        ]
        focused_test_contract_sha256 = canonical_sha256(focused_test_contract)
        required_classes = validation_classes(row)

        artifact_ids = {"parity-matrix"}
        artifact_ids.add("client-owner-inventory")
        if row["status"] in ROUTED_STATUSES:
            artifact_ids.add("feature-ledger")
        for reference in refs:
            relative = reference_path(reference)
            path = repo / relative
            if not path.is_file():
                continue
            artifact_id = f"repo:{relative}"
            artifacts.setdefault(artifact_id, repo_artifact(repo, artifact_id, relative))
            artifact_ids.add(artifact_id)
        for artifact_id in test_source_artifact_ids:
            relative = artifact_id.removeprefix("repo:")
            artifacts.setdefault(artifact_id, repo_artifact(repo, artifact_id, relative))
            artifact_ids.add(artifact_id)
        external_ids = external_artifact_ids(row)
        artifact_ids.update(external_ids)
        if has_executable_tests:
            artifact_ids.add("full-gate")

        scenarios: list[dict[str, object]] = []
        if has_executable_tests:
            scenario_id = f"focused-{mechanism_id}"
            scenario_artifact_ids = sorted(
                {"full-gate"}
                | set(test_source_artifact_ids)
            )
            scenarios.append(
                {
                    "artifact_ids": scenario_artifact_ids,
                    "kind": differential_kind(row),
                    "scenario_id": scenario_id,
                    "status": "passed"
                    if focused_current
                    else "invalid",
                }
            )
        for artifact_id in external_ids:
            detailed_ids = artifacts[artifact_id].get("scenario_ids", [])
            suffixes = [None]
            if isinstance(detailed_ids, list):
                suffixes.extend(
                    item for item in detailed_ids if isinstance(item, str)
                )
            for suffix in suffixes:
                scenario_id = f"receipt-{artifact_id}"
                if suffix is not None:
                    scenario_id += f":{suffix}"
                scenarios.append(
                    {
                        "artifact_ids": [artifact_id],
                        "kind": artifacts[artifact_id]["kind"],
                        "scenario_id": scenario_id,
                        "status": "passed"
                        if artifacts[artifact_id]["valid"]
                        else "invalid",
                    }
                )
        if row["status"] in {"not-needed", "deferred-external"}:
            scenarios.append(
                {
                    "artifact_ids": ["feature-ledger", "client-owner-inventory"],
                    "kind": "scope-ruling",
                    "scenario_id": f"scope-{mechanism_id}",
                    "status": "passed" if mechanism_id in ledger_ids else "invalid",
                }
            )
        if not scenarios:
            scenarios.append(
                {
                    "artifact_ids": ["parity-matrix", "client-owner-inventory"],
                    "kind": differential_kind(row),
                    "scenario_id": f"missing-{mechanism_id}",
                    "status": "invalid",
                }
            )

        reported_tests = tests
        if not reported_tests:
            reported_tests = [
                {
                    "command": None,
                    "ignored": False,
                    "reference": None,
                    "result": "not-applicable"
                    if row["status"] in {"not-needed", "deferred-external"}
                    else "missing",
                }
            ]

        reasons: list[str] = []
        if not source_hashes and not allows_empty_vanilla_source_hashes(mechanism_id):
            reasons.append("no hashed Vanilla owner source was resolved")
        if row["status"] in ROUTED_STATUSES and mechanism_id not in ledger_ids:
            reasons.append("matrix status has no bilateral ledger route")
        missing_refs = sorted(
            reference for reference in refs if not (repo / reference_path(reference)).is_file()
        )
        if missing_refs:
            reasons.append("missing BBB evidence references: " + ", ".join(missing_refs))
        invalid_repo_artifacts = sorted(
            artifact_id
            for artifact_id in artifact_ids
            if artifacts[artifact_id].get("kind") == "repo-evidence"
            and artifacts[artifact_id].get("valid") is not True
        )
        if invalid_repo_artifacts:
            reasons.append(
                "required repo evidence is invalid: "
                + ", ".join(invalid_repo_artifacts)
            )
        for artifact_id in external_ids:
            if not artifacts[artifact_id]["valid"]:
                reasons.append(
                    f"required {artifact_id} receipt is invalid: "
                    + "; ".join(artifacts[artifact_id]["invalid_reasons"])
                )
        if row["status"] == "equivalent":
            changed_evidence = sorted(
                item["path"]
                for item in dependency_closure
                if item["path"] in changed_paths
                and not is_closure_metadata_path(str(item["path"]))
                and item["path"] not in VALIDATION_ONLY_PATHS
            )
            if changed_evidence:
                reasons.append(
                    "BBB evidence changed after the gated commit: "
                    + ", ".join(changed_evidence)
                )
            passed_focused = any(
                scenario["scenario_id"] == f"focused-{mechanism_id}"
                and scenario["status"] == "passed"
                for scenario in scenarios
            )
            passed_external = all(artifacts[artifact_id]["valid"] for artifact_id in external_ids)
            if not passed_focused:
                reasons.append("equivalent row lacks current passing focused evidence")
            if not passed_external:
                reasons.append("equivalent row lacks every required formal receipt")
        elif row["status"] == "not-needed":
            if not ledger_basis or "Reopen" not in ledger_basis:
                reasons.append("not-needed ruling lacks a ledger reopen condition")
        elif row["status"] == "deferred-external":
            if not ledger_basis or "Reopen when" not in ledger_basis:
                reasons.append("deferred ruling lacks an exact Reopen when condition")
        else:
            if not has_executable_tests and not external_ids:
                reasons.append("focused decision evidence is missing")
            reasons.append(f"mechanism remains open with matrix status {row['status']}")

        open_reasons = derived_open_reasons(
            status=row["status"],
            owner_open_gaps=owner_open_gaps,
            has_implementation_evidence=bool(refs or owner_implementations),
            focused_current=focused_current,
            external_ids=external_ids,
            artifacts=artifacts,
            workflow_stage=workflow_stage,
        )

        valid = not reasons
        if row["status"] == "equivalent":
            conclusion = "equivalent" if valid else "invalid-equivalent"
        elif row["status"] == "not-needed":
            conclusion = "not-needed" if valid else "invalid-scope-ruling"
        elif row["status"] == "deferred-external":
            conclusion = "deferred-external" if valid else "invalid-deferred-ruling"
        else:
            conclusion = "open"
        owner_ids = sorted(
            owner["owner_id"]
            for owner in owner_rows
            if isinstance(owner.get("owner_id"), str)
        )
        record = {
            "artifact_ids": sorted(artifact_ids),
            "bbb": {
                "changed_dependency_paths": sorted(
                    item["path"]
                    for item in dependency_closure
                    if item["path"] in changed_paths
                ),
                "dependency_closure": dependency_closure,
                "dependency_closure_sha256": dependency_closure_sha256,
                "evidence_commit": evidence_commit,
                "evidence_references": refs,
                "focused_test_contract_sha256": focused_test_contract_sha256,
                "owner": row["bbb_owner"],
                "owner_contract_sha256": canonical_sha256(
                    {
                        "dependency_closure_sha256": dependency_closure_sha256,
                        "owner": row["bbb_owner"],
                        "required_state": row["required_state"],
                    }
                ),
            },
            "differential_scenarios": scenarios,
            "final_conclusion": conclusion,
            "focused_tests": reported_tests,
            "mechanism_id": mechanism_id,
            "open_reasons": sorted(set(open_reasons)),
            "required_validation_classes": required_classes,
            "scope_ruling_sha256": (
                canonical_sha256(ledger_basis)
                if row["status"] in {"not-needed", "deferred-external"}
                else None
            ),
            "matrix": {
                "domain": row["domain"],
                "line": int(row["line"]),
                "required_state_projection": row["required_state"],
                "route_reopen_condition": row["route"],
                "status": row["status"],
            },
            "valid": valid,
            "validity_reasons": reasons,
            "vanilla": {
                "corpus_source_sha256": corpus_source_sha256,
                "inventory_owner_count": len(owner_ids),
                "inventory_owner_ids": owner_ids,
                "inventory_owner_set_sha256": canonical_sha256(owner_ids),
                "matrix_owner": row["vanilla_owner"],
                "source_hashes": source_hashes,
            },
        }
        records.append(record)
        for artifact_id in artifact_ids:
            artifact_mechanisms[artifact_id].add(mechanism_id)
        for scenario in scenarios:
            scenario_id = str(scenario["scenario_id"])
            scenario_mechanisms[scenario_id].add(mechanism_id)
            scenario_artifacts[scenario_id].update(
                str(item) for item in scenario["artifact_ids"]
            )
            scenario_kinds[scenario_id].add(str(scenario["kind"]))
            scenario_statuses[scenario_id].add(str(scenario["status"]))

    artifact_values = []
    for artifact_id in sorted(artifacts):
        artifact = dict(artifacts[artifact_id])
        artifact["mechanism_ids"] = sorted(artifact_mechanisms.get(artifact_id, set()))
        artifact_values.append(artifact)

    change_impact = change_impact_plan(
        records, changed_paths, evidence_commit, artifact_values
    )
    affected_by_change = set(change_impact["functional_affected_mechanism_ids"])
    for record in records:
        if (
            record.get("matrix", {}).get("status") == "equivalent"
            and record.get("mechanism_id") in affected_by_change
        ):
            reason = "changed-path plan requires current validation"
            if reason not in record["validity_reasons"]:
                record["validity_reasons"].append(reason)
            record["valid"] = False
            record["final_conclusion"] = "invalid-equivalent"

    scenario_values = []
    for scenario_id, mechanism_ids in sorted(scenario_mechanisms.items()):
        kinds = scenario_kinds[scenario_id]
        statuses = scenario_statuses[scenario_id]
        if len(kinds) != 1 or len(statuses) != 1:
            raise AcceptanceError(f"inconsistent shared scenario metadata: {scenario_id}")
        scenario_values.append(
            {
                "artifact_ids": sorted(scenario_artifacts[scenario_id]),
                "kind": next(iter(kinds)),
                "mechanism_ids": sorted(mechanism_ids),
                "scenario_id": scenario_id,
                "status": next(iter(statuses)),
            }
        )
    status_counts: dict[str, int] = defaultdict(int)
    invalid_ids: list[str] = []
    false_equivalents: list[str] = []
    for record in records:
        status = str(record["matrix"]["status"])
        status_counts[status] += 1
        if not record["valid"]:
            invalid_ids.append(str(record["mechanism_id"]))
            if status == "equivalent":
                false_equivalents.append(str(record["mechanism_id"]))
    invalid_ids.sort()
    false_equivalents.sort()
    generator_inputs = []
    for relative in (
        "scripts/parity_acceptance.py",
        "scripts/bbb_tools/executable_identity.py",
        MATRIX_PATH.as_posix(),
        INVENTORY_PATH.as_posix(),
        LEDGER_PATH.as_posix(),
    ):
        path = repo / relative
        generator_inputs.append({"path": relative, "sha256": sha256_file(path)})
    semantic_contract_sha256 = parity_semantic_contract_sha256(records)
    blocking_ids = implementation_blocking_ids(records)
    implementation_ready = not blocking_ids
    code_freeze_ready = (
        implementation_ready
        and artifacts["full-gate"]["valid"] is True
        and not change_impact["functional_changed_paths"]
        and not change_impact["functional_affected_mechanism_ids"]
        and not change_impact["validation_only_changed_paths"]
    )
    return {
        "artifacts": artifact_values,
        "bbb_evidence_commit": evidence_commit,
        "commands": {
            "change_plan": "scripts/python python scripts/parity_acceptance.py plan --base-commit <commit>",
            "final_exact_head_check": "scripts/python python scripts/parity_acceptance.py check --require-artifacts --require-complete --require-final-head",
            "portable_check": "scripts/python python scripts/parity_acceptance.py check",
            "strict_artifact_check": "scripts/python python scripts/parity_acceptance.py check --require-artifacts --require-complete",
        },
        "generated_by": "scripts/parity_acceptance.py",
        "generator_inputs": generator_inputs,
        "change_impact": change_impact,
        "mechanisms": records,
        "scenarios": scenario_values,
        "schema_version": SCHEMA_VERSION,
        "summary": {
            "declared_scope_complete": not invalid_ids,
            "false_equivalent_ids": false_equivalents,
            "invalid_ids": invalid_ids,
            "mechanism_count": len(records),
            "status_counts": dict(sorted(status_counts.items())),
            "valid_count": len(records) - len(invalid_ids),
        },
        "vanilla_version": "26.1",
        "workflow": {
            "code_freeze_ready": code_freeze_ready,
            "consistency_gate_passed": consistency_gate_passed,
            "formal_performance_allowed": consistency_gate_passed
            and workflow_stage_index(workflow_stage)
            >= workflow_stage_index("final-performance"),
            "implementation_blocking_ids": blocking_ids,
            "implementation_ready": implementation_ready,
            "semantic_contract_sha256": semantic_contract_sha256,
            "stage": workflow_stage,
            "stage_order": list(WORKFLOW_STAGES),
        },
    }


def validate_report(
    report: dict[str, object],
    *,
    repo: Path,
    source_root: Path,
    enforce_current: bool,
    require_artifacts: bool = False,
    require_complete: bool = False,
    require_final_head: bool = False,
) -> list[str]:
    issues: list[str] = []
    if report.get("schema_version") != SCHEMA_VERSION:
        issues.append("unsupported parity acceptance schema")
    if report.get("vanilla_version") != "26.1":
        issues.append("parity acceptance report is not Vanilla 26.1")
    matrix_path = repo / MATRIX_PATH
    ledger_path = repo / LEDGER_PATH
    rows = parse_matrix(matrix_path)
    rows_by_id = {row["mechanism_id"]: row for row in rows}
    matrix_ids = [row["mechanism_id"] for row in rows]
    inventory = json.loads((repo / INVENTORY_PATH).read_text(encoding="utf-8"))
    inventory_mechanisms = inventory.get("mechanisms", {})
    inventory_owners = inventory.get("owners", [])
    inventory_source_files = inventory.get("source_files", [])
    if (
        not isinstance(inventory_mechanisms, dict)
        or not isinstance(inventory_owners, list)
        or not isinstance(inventory_source_files, list)
    ):
        return [*issues, "client mechanism inventory is malformed"]
    inventory_owner_ids: dict[str, list[str]] = defaultdict(list)
    inventory_owners_by_mechanism: dict[str, list[dict[str, object]]] = defaultdict(
        list
    )
    for owner in inventory_owners:
        if (
            isinstance(owner, dict)
            and isinstance(owner.get("mechanism_id"), str)
            and isinstance(owner.get("owner_id"), str)
        ):
            inventory_owner_ids[owner["mechanism_id"]].append(owner["owner_id"])
            inventory_owners_by_mechanism[owner["mechanism_id"]].append(owner)
    for owner_ids in inventory_owner_ids.values():
        owner_ids.sort()
    inventory_hashes = {
        item["path"]: item["sha256"]
        for item in inventory_source_files
        if isinstance(item, dict)
        and isinstance(item.get("path"), str)
        and isinstance(item.get("sha256"), str)
    }
    corpus_source_sha256 = canonical_sha256(inventory_source_files)
    ledger_by_id, ledger_ids = ledger_sections(ledger_path.read_text(encoding="utf-8"))
    class_sources = source_index(source_root)
    evidence_commit = report.get("bbb_evidence_commit")
    if not isinstance(evidence_commit, str) or not re.fullmatch(
        r"[0-9a-f]{40}", evidence_commit
    ):
        issues.append("report has no exact BBB evidence commit")
        evidence_commit = ""
    changed_paths = changed_repo_paths(repo, evidence_commit) if evidence_commit else set()
    changed_paths.discard(REPORT_PATH.as_posix())
    records = report.get("mechanisms")
    artifacts = report.get("artifacts")
    scenarios = report.get("scenarios")
    if not isinstance(records, list) or not isinstance(artifacts, list) or not isinstance(scenarios, list):
        return [*issues, "report mechanisms/artifacts/scenarios are malformed"]
    record_ids = [
        record.get("mechanism_id") for record in records if isinstance(record, dict)
    ]
    if record_ids != matrix_ids:
        issues.append("report mechanism order/set differs from the canonical matrix")
    artifact_map = {
        item.get("artifact_id"): item for item in artifacts if isinstance(item, dict)
    }
    scenario_map = {
        item.get("scenario_id"): item for item in scenarios if isinstance(item, dict)
    }
    if len(artifact_map) != len(artifacts):
        issues.append("duplicate or malformed artifact ids")
    if len(scenario_map) != len(scenarios):
        issues.append("duplicate or malformed scenario ids")
    workflow = report.get("workflow")
    if not isinstance(workflow, dict):
        issues.append("report workflow is malformed")
        workflow = {}
    stage = workflow.get("stage")
    if stage not in WORKFLOW_STAGES:
        issues.append("report workflow stage is invalid")
        stage = "implementation"
    if workflow.get("stage_order") != list(WORKFLOW_STAGES):
        issues.append("report workflow stage order is stale")
    reverse_artifacts: dict[str, set[str]] = defaultdict(set)
    reverse_scenarios: dict[str, set[str]] = defaultdict(set)
    reverse_scenario_artifacts: dict[str, set[str]] = defaultdict(set)
    reverse_scenario_kinds: dict[str, set[str]] = defaultdict(set)
    reverse_scenario_statuses: dict[str, set[str]] = defaultdict(set)
    matrix_status = {row["mechanism_id"]: row["status"] for row in rows}
    for record in records:
        if not isinstance(record, dict) or not isinstance(record.get("mechanism_id"), str):
            issues.append("malformed mechanism record")
            continue
        mechanism_id = record["mechanism_id"]
        row = rows_by_id.get(mechanism_id)
        if row is None:
            issues.append(f"unknown mechanism record: {mechanism_id}")
            continue
        matrix = record.get("matrix")
        if not isinstance(matrix, dict) or matrix.get("status") != matrix_status.get(mechanism_id):
            issues.append(f"stale matrix status for {mechanism_id}")
        elif matrix != {
            "domain": row["domain"],
            "line": int(row["line"]),
            "required_state_projection": row["required_state"],
            "route_reopen_condition": row["route"],
            "status": row["status"],
        }:
            issues.append(f"stale matrix projection for {mechanism_id}")
        record_artifacts = record.get("artifact_ids", [])
        if not isinstance(record_artifacts, list) or not record_artifacts:
            issues.append(f"mechanism has no artifacts: {mechanism_id}")
            record_artifacts = []
        for artifact_id in record_artifacts:
            if artifact_id not in artifact_map:
                issues.append(f"unknown artifact {artifact_id!r} for {mechanism_id}")
            else:
                reverse_artifacts[str(artifact_id)].add(mechanism_id)
        record_scenarios = record.get("differential_scenarios", [])
        if not isinstance(record_scenarios, list) or not record_scenarios:
            issues.append(f"mechanism has no differential scenario: {mechanism_id}")
            record_scenarios = []
        for scenario in record_scenarios:
            if not isinstance(scenario, dict) or scenario.get("scenario_id") not in scenario_map:
                issues.append(f"unknown scenario for {mechanism_id}")
                continue
            scenario_id = str(scenario["scenario_id"])
            reverse_scenarios[scenario_id].add(mechanism_id)
            reverse_scenario_kinds[scenario_id].add(str(scenario.get("kind")))
            reverse_scenario_statuses[scenario_id].add(str(scenario.get("status")))
            for artifact_id in scenario.get("artifact_ids", []):
                if artifact_id not in artifact_map:
                    issues.append(
                        f"unknown scenario artifact {artifact_id!r} for {mechanism_id}"
                    )
                reverse_scenario_artifacts[scenario_id].add(str(artifact_id))
        focused_tests = record.get("focused_tests")
        if not isinstance(focused_tests, list) or not focused_tests:
            issues.append(f"mechanism has no focused-test result: {mechanism_id}")
            focused_tests = []
        expected_classes = validation_classes(row)
        if record.get("required_validation_classes") != expected_classes:
            issues.append(f"stale validation classes for {mechanism_id}")
        open_reasons = record.get("open_reasons")
        if not isinstance(open_reasons, list) or any(
            reason not in OPEN_REASON_CODES for reason in open_reasons
        ):
            issues.append(f"malformed open reasons for {mechanism_id}")
            open_reasons = []
        if row["status"] in OPEN_STATUSES and not open_reasons:
            issues.append(f"open mechanism lacks machine-readable reason: {mechanism_id}")
        if row["status"] not in OPEN_STATUSES and open_reasons:
            issues.append(f"closed mechanism retains open reasons: {mechanism_id}")
        expected_scope_ruling_sha256 = (
            canonical_sha256(ledger_by_id.get(mechanism_id, ""))
            if row["status"] in {"not-needed", "deferred-external"}
            else None
        )
        if record.get("scope_ruling_sha256") != expected_scope_ruling_sha256:
            issues.append(f"stale scope-ruling hash for {mechanism_id}")
        vanilla = record.get("vanilla")
        if not isinstance(vanilla, dict) or not isinstance(
            vanilla.get("corpus_source_sha256"), str
        ):
            issues.append(f"mechanism lacks a Vanilla corpus hash: {mechanism_id}")
        else:
            expected_owner_ids = inventory_owner_ids.get(mechanism_id, [])
            if vanilla.get("corpus_source_sha256") != corpus_source_sha256:
                issues.append(f"stale Vanilla corpus hash for {mechanism_id}")
            if vanilla.get("matrix_owner") != row["vanilla_owner"]:
                issues.append(f"stale Vanilla matrix owner for {mechanism_id}")
            if vanilla.get("inventory_owner_ids") != expected_owner_ids:
                issues.append(f"stale Vanilla owner mapping for {mechanism_id}")
            if vanilla.get("inventory_owner_count") != len(expected_owner_ids):
                issues.append(f"stale Vanilla owner count for {mechanism_id}")
            if vanilla.get("inventory_owner_set_sha256") != canonical_sha256(
                expected_owner_ids
            ):
                issues.append(f"stale Vanilla owner-set hash for {mechanism_id}")
            references = evidence_references(
                repo,
                matrix_path,
                row,
                inventory_mechanisms.get(mechanism_id)
                if isinstance(inventory_mechanisms.get(mechanism_id), dict)
                else None,
            )
            evidence_basis_parts = [row["evidence"]]
            for reference in references:
                path = repo / reference_path(reference)
                if (
                    path.is_file()
                    and path.suffix == ".md"
                    and path.resolve() != ledger_path.resolve()
                ):
                    evidence_basis_parts.append(path.read_text(encoding="utf-8"))
            try:
                expected_source_hashes = mechanism_source_hashes(
                    mechanism_id=mechanism_id,
                    row=row,
                    owners=inventory_owners_by_mechanism.get(mechanism_id, []),
                    inventory_hashes=inventory_hashes,
                    class_sources=class_sources,
                    source_root=source_root,
                    ledger_basis=ledger_by_id.get(mechanism_id, ""),
                    evidence_basis="\n".join(evidence_basis_parts),
                )
            except AcceptanceError as error:
                issues.append(f"invalid Vanilla source mapping for {mechanism_id}: {error}")
            else:
                if vanilla.get("source_hashes") != expected_source_hashes:
                    issues.append(f"stale Vanilla source mapping for {mechanism_id}")
            if (
                not vanilla.get("source_hashes")
                and not allows_empty_vanilla_source_hashes(mechanism_id)
                and record.get("valid") is True
            ):
                issues.append(f"mechanism lacks hashed Vanilla owner source: {mechanism_id}")
        bbb = record.get("bbb")
        if (
            not isinstance(bbb, dict)
            or not isinstance(bbb.get("evidence_commit"), str)
            or not isinstance(bbb.get("owner"), str)
        ):
            issues.append(f"mechanism lacks BBB commit/owner evidence: {mechanism_id}")
        else:
            if bbb.get("evidence_commit") != evidence_commit:
                issues.append(f"stale BBB evidence commit for {mechanism_id}")
            if bbb.get("owner") != row["bbb_owner"]:
                issues.append(f"stale BBB owner for {mechanism_id}")
            references = bbb.get("evidence_references")
            if not isinstance(references, list):
                issues.append(f"malformed BBB evidence references for {mechanism_id}")
                references = []
            missing_references = sorted(
                reference
                for reference in references
                if not isinstance(reference, str)
                or not (repo / reference_path(reference)).is_file()
            )
            if missing_references:
                issues.append(f"missing BBB evidence for {mechanism_id}")
            closure = bbb.get("dependency_closure")
            if not isinstance(closure, list):
                issues.append(f"malformed BBB dependency closure for {mechanism_id}")
                closure = []
            dependency_roles: dict[str, set[str]] = defaultdict(set)
            for reference in references:
                if isinstance(reference, str):
                    dependency_roles[reference_path(reference)].add("matrix-evidence")
            for owner in inventory_owners_by_mechanism.get(mechanism_id, []):
                owner_evidence = owner.get("bbb_evidence")
                if not isinstance(owner_evidence, dict):
                    continue
                for reference in owner_evidence.get("implementation", []):
                    if isinstance(reference, str):
                        dependency_roles[reference_path(reference)].add(
                            "owner-implementation"
                        )
                for reference in owner_evidence.get("tests", []):
                    if isinstance(reference, str):
                        dependency_roles[reference_path(reference)].add(
                            "owner-focused-test"
                        )
            for test in focused_tests:
                if not isinstance(test, dict) or test.get("result") in {
                    "not-applicable",
                    "missing",
                }:
                    continue
                reference = test.get("reference")
                if isinstance(reference, str):
                    dependency_roles[reference_path(reference)].add("focused-test")
                for artifact_id in test.get("source_artifact_ids", []):
                    if isinstance(artifact_id, str):
                        dependency_roles[artifact_id.removeprefix("repo:")].add(
                            "focused-test-set"
                        )
            expected_closure = hashed_repo_paths(repo, dependency_roles)
            if closure != expected_closure:
                issues.append(f"stale BBB dependency closure for {mechanism_id}")
            for item in closure:
                if (
                    not isinstance(item, dict)
                    or not isinstance(item.get("path"), str)
                    or not isinstance(item.get("roles"), list)
                    or not isinstance(item.get("sha256"), str)
                ):
                    issues.append(f"malformed BBB dependency item for {mechanism_id}")
                    continue
                path = repo / item["path"]
                if not path.is_file() or sha256_file(path) != item["sha256"]:
                    issues.append(
                        f"stale BBB dependency {item['path']} for {mechanism_id}"
                    )
            closure_sha256 = canonical_sha256(closure)
            if bbb.get("dependency_closure_sha256") != closure_sha256:
                issues.append(f"stale BBB dependency closure hash for {mechanism_id}")
            expected_changed_dependencies = sorted(
                item["path"]
                for item in closure
                if isinstance(item, dict)
                and isinstance(item.get("path"), str)
                and item["path"] in changed_paths
            )
            if bbb.get("changed_dependency_paths") != expected_changed_dependencies:
                issues.append(f"stale changed BBB dependencies for {mechanism_id}")
            focused_contract = [
                {
                    key: test.get(key)
                    for key in (
                        "command",
                        "evidence_set_count",
                        "evidence_set_sha256",
                        "ignored",
                        "reference",
                        "source_artifact_ids",
                        "source_sha256",
                    )
                    if key in test
                }
                for test in focused_tests
                if isinstance(test, dict)
                and test.get("result") not in {"not-applicable", "missing"}
            ]
            if bbb.get("focused_test_contract_sha256") != canonical_sha256(
                focused_contract
            ):
                issues.append(f"stale focused-test contract for {mechanism_id}")
            expected_owner_contract = canonical_sha256(
                {
                    "dependency_closure_sha256": closure_sha256,
                    "owner": row["bbb_owner"],
                    "required_state": row["required_state"],
                }
            )
            if bbb.get("owner_contract_sha256") != expected_owner_contract:
                issues.append(f"stale BBB owner contract for {mechanism_id}")
        validity_reasons = record.get("validity_reasons")
        if not isinstance(validity_reasons, list) or any(
            not isinstance(reason, str) for reason in validity_reasons
        ):
            issues.append(f"malformed validity reasons for {mechanism_id}")
            validity_reasons = []
        if record.get("valid") is not (not validity_reasons):
            issues.append(f"inconsistent validity result for {mechanism_id}")

        status = matrix_status.get(mechanism_id)
        owner_rows = inventory_owners_by_mechanism.get(mechanism_id, [])
        owner_open_gaps = sum(
            1
            for owner in owner_rows
            if isinstance(owner.get("bbb_evidence"), dict)
            and isinstance(owner["bbb_evidence"].get("open_gap"), dict)
        )
        owner_implementations = [
            reference
            for owner in owner_rows
            if isinstance(owner.get("bbb_evidence"), dict)
            for reference in owner["bbb_evidence"].get("implementation", [])
            if isinstance(reference, str)
        ]
        focused_current = any(
            isinstance(scenario, dict)
            and scenario.get("scenario_id") == f"focused-{mechanism_id}"
            and scenario.get("status") == "passed"
            for scenario in record_scenarios
        )
        expected_open_reasons = derived_open_reasons(
            status=str(status),
            owner_open_gaps=owner_open_gaps,
            has_implementation_evidence=bool(
                (bbb.get("evidence_references") if isinstance(bbb, dict) else [])
                or owner_implementations
            ),
            focused_current=focused_current,
            external_ids=external_artifact_ids(row),
            artifacts={
                str(artifact_id): artifact
                for artifact_id, artifact in artifact_map.items()
                if isinstance(artifact_id, str) and isinstance(artifact, dict)
            },
            workflow_stage=str(stage),
        )
        if record.get("open_reasons") != expected_open_reasons:
            issues.append(f"stale open reasons for {mechanism_id}")
        if status in ROUTED_STATUSES and mechanism_id not in ledger_ids:
            issues.append(f"unrouted matrix status for {mechanism_id}")
        if status == "equivalent":
            dependency_paths = {
                str(item["path"])
                for item in (
                    bbb.get("dependency_closure", []) if isinstance(bbb, dict) else []
                )
                if isinstance(item, dict) and isinstance(item.get("path"), str)
            }
            if sorted(
                path
                for path in dependency_paths & changed_paths
                if not is_closure_metadata_path(path)
                and path not in VALIDATION_ONLY_PATHS
            ):
                issues.append(f"ungated BBB evidence for {mechanism_id}")
            passing_focused = False
            for scenario in record_scenarios:
                if not isinstance(scenario, dict) or scenario.get("status") != "passed":
                    continue
                scenario_id = scenario.get("scenario_id")
                if not isinstance(scenario_id, str) or not scenario_id.startswith(
                    "focused-"
                ):
                    continue
                artifact_ids = scenario.get("artifact_ids", [])
                if isinstance(artifact_ids, list) and artifact_ids and all(
                    isinstance(artifact_map.get(artifact_id), dict)
                    and artifact_map[artifact_id].get("valid") is True
                    for artifact_id in artifact_ids
                ):
                    passing_focused = True
            required_external = external_artifact_ids(row)
            passing_formal = all(
                isinstance(artifact_map.get(artifact_id), dict)
                and artifact_map[artifact_id].get("valid") is True
                and artifact_map[artifact_id].get("evidence_class") == "formal"
                for artifact_id in required_external
            )
            passing_repo_evidence = all(
                isinstance(artifact_map.get(artifact_id), dict)
                and artifact_map[artifact_id].get("valid") is True
                for artifact_id in record_artifacts
                if isinstance(artifact_map.get(artifact_id), dict)
                and artifact_map[artifact_id].get("kind") == "repo-evidence"
            )
            if (
                not passing_focused
                or not passing_formal
                or not passing_repo_evidence
                or record.get("valid") is not True
            ):
                issues.append(f"false equivalent: {mechanism_id}")
        elif status == "not-needed":
            if "Reopen" not in ledger_by_id.get(mechanism_id, ""):
                issues.append(f"not-needed ruling lacks reopen condition: {mechanism_id}")
        elif status == "deferred-external":
            if "Reopen when" not in ledger_by_id.get(mechanism_id, ""):
                issues.append(f"deferred ruling lacks reopen condition: {mechanism_id}")
        if (
            status in OPEN_STATUSES - {"deferred-external"}
            and record.get("final_conclusion") != "open"
        ):
            issues.append(f"open mechanism has a closed conclusion: {mechanism_id}")
        expected_conclusion = {
            "equivalent": "equivalent" if record.get("valid") is True else "invalid-equivalent",
            "not-needed": "not-needed" if record.get("valid") is True else "invalid-scope-ruling",
            "deferred-external": "deferred-external" if record.get("valid") is True else "invalid-deferred-ruling",
        }.get(status, "open")
        if record.get("final_conclusion") != expected_conclusion:
            issues.append(f"inconsistent final conclusion for {mechanism_id}")
    for artifact_id, artifact in artifact_map.items():
        invalid_reasons = artifact.get("invalid_reasons")
        if not isinstance(invalid_reasons, list) or any(
            not isinstance(reason, str) for reason in invalid_reasons
        ):
            issues.append(f"malformed artifact validity for {artifact_id}")
        elif artifact.get("valid") is not (not invalid_reasons):
            issues.append(f"inconsistent artifact validity for {artifact_id}")
        for reason in projection_reuse_reasons(repo, artifact, evidence_commit):
            issues.append(f"{reason}: {artifact_id}")
        artifact_kind = artifact.get("kind")
        if artifact_kind not in {"repo-evidence", "gate"}:
            if artifact.get("evidence_class") not in {
                "formal",
                "diagnostic",
                "absent",
                "invalid",
            }:
                issues.append(f"malformed evidence class for {artifact_id}")
            capture_contract = artifact.get("capture_contract")
            if capture_contract is not None:
                if not isinstance(capture_contract, dict):
                    issues.append(f"malformed capture contract for {artifact_id}")
                else:
                    contract_without_hash = {
                        key: value
                        for key, value in capture_contract.items()
                        if key != "sha256"
                    }
                    if capture_contract.get("sha256") != canonical_sha256(
                        contract_without_hash
                    ):
                        issues.append(f"stale capture contract hash for {artifact_id}")
                    for identity_role in ("tools", "inputs"):
                        identities = capture_contract.get(identity_role)
                        if not isinstance(identities, list):
                            issues.append(
                                f"malformed capture {identity_role} for {artifact_id}"
                            )
                            continue
                        for identity in identities:
                            if not isinstance(identity, dict) or not isinstance(
                                identity.get("path"), str
                            ):
                                issues.append(
                                    f"malformed capture {identity_role} identity for {artifact_id}"
                                )
                                continue
                            path = repo / identity["path"]
                            if not path.is_file() or sha256_file(path) != identity.get(
                                "current_sha256"
                            ):
                                issues.append(
                                    f"stale capture {identity_role} identity for {artifact_id}: {identity['path']}"
                                )
                            if identity.get("capture_sha256") != identity.get(
                                "current_sha256"
                            ) and not any(
                                identity["path"] in reason
                                for reason in (
                                    invalid_reasons
                                    if isinstance(invalid_reasons, list)
                                    else []
                                )
                                if isinstance(reason, str)
                            ):
                                issues.append(
                                    f"changed capture {identity_role} lacks invalidation for {artifact_id}: {identity['path']}"
                                )
            if artifact.get("valid") is True and artifact.get("evidence_class") != "formal":
                issues.append(f"diagnostic artifact claims formal validity: {artifact_id}")
        if artifact.get("mechanism_ids") != sorted(reverse_artifacts.get(str(artifact_id), set())):
            issues.append(f"stale reverse Mechanism-ID mapping for artifact {artifact_id}")
        path_value = artifact.get("path")
        digest = artifact.get("sha256")
        if enforce_current and artifact.get("kind") == "repo-evidence" and isinstance(path_value, str):
            path = repo / path_value
            if not path.is_file() or sha256_file(path) != digest:
                issues.append(f"stale repo artifact hash: {artifact_id}")
            refreshed = repo_artifact(repo, str(artifact_id), path_value)
            if artifact.get("generator_inputs") != refreshed.get(
                "generator_inputs"
            ):
                issues.append(
                    f"stale repo artifact generator-input projection: {artifact_id}"
                )
            if artifact.get("invalid_reasons") != refreshed.get("invalid_reasons"):
                issues.append(f"stale repo artifact validity: {artifact_id}")
            if path.is_file() and path.suffix == ".json":
                try:
                    repo_value = json.loads(path.read_text(encoding="utf-8"))
                except (UnicodeDecodeError, json.JSONDecodeError):
                    repo_value = None
                declared_ids = (
                    repo_value.get("mechanism_ids")
                    if isinstance(repo_value, dict)
                    else None
                )
                if declared_ids is not None:
                    if not isinstance(declared_ids, list) or any(
                        not isinstance(mechanism_id, str)
                        for mechanism_id in declared_ids
                    ):
                        issues.append(
                            f"malformed declared Mechanism-ID mapping for artifact {artifact_id}"
                        )
                    else:
                        unknown_ids = sorted(set(declared_ids) - set(matrix_ids))
                        if unknown_ids:
                            issues.append(
                                f"unknown declared Mechanism ID for artifact {artifact_id}: "
                                + ", ".join(unknown_ids)
                            )
                        if sorted(set(declared_ids)) != sorted(
                            reverse_artifacts.get(str(artifact_id), set())
                        ):
                            issues.append(
                                f"declared/reverse Mechanism-ID mismatch for artifact {artifact_id}"
                            )
        if require_artifacts and artifact.get("kind") != "repo-evidence":
            if not isinstance(path_value, str):
                issues.append(f"external artifact path is absent: {artifact_id}")
            else:
                path = Path(path_value)
                if not path.is_absolute():
                    path = repo / path
                if not path.is_file():
                    issues.append(f"external artifact is absent: {artifact_id}")
                elif artifact_kind == "gate":
                    issues.extend(
                        current_gate_receipt_issues(artifact, path, str(artifact_id))
                    )
                elif not isinstance(digest, str) or sha256_file(path) != digest:
                    issues.append(f"external artifact hash is stale: {artifact_id}")
            provenance = artifact.get("bbb_provenance")
            if isinstance(provenance, dict):
                for label in ("capture", "evidence"):
                    binary = provenance.get(f"{label}_binary")
                    if not isinstance(binary, dict):
                        issues.append(
                            f"external artifact {artifact_id} lacks {label} binary provenance"
                        )
                        continue
                    binary_path_value = binary.get("path")
                    if not isinstance(binary_path_value, str):
                        issues.append(
                            f"external artifact {artifact_id} has no {label} binary path"
                        )
                        continue
                    binary_path = Path(binary_path_value)
                    if not binary_path.is_file():
                        issues.append(
                            f"external artifact {artifact_id} {label} binary is absent"
                        )
                    elif sha256_file(binary_path) != binary.get("sha256"):
                        issues.append(
                            f"external artifact {artifact_id} {label} binary hash is stale"
                        )
            capture_contract = artifact.get("capture_contract")
            binary_identity = (
                capture_contract.get("binary_load_identity")
                if isinstance(capture_contract, dict)
                else None
            )
            if (
                isinstance(binary_identity, dict)
                and binary_identity.get("identity_kind") == "file-sha256"
            ):
                binary_path_value = binary_identity.get("binary")
                if not isinstance(binary_path_value, str):
                    issues.append(
                        f"external artifact {artifact_id} has no BBB binary path"
                    )
                else:
                    binary_path = Path(binary_path_value)
                    if not binary_path.is_file():
                        issues.append(
                            f"external artifact {artifact_id} BBB binary is absent"
                        )
                    elif sha256_file(binary_path) != binary_identity.get(
                        "binary_sha256"
                    ):
                        issues.append(
                            f"external artifact {artifact_id} BBB binary hash is stale"
                        )
            if isinstance(path_value, str) and artifact_kind != "gate":
                artifact_path = Path(path_value)
                if not artifact_path.is_absolute():
                    artifact_path = repo / artifact_path
                refreshed = external_artifact(
                    repo=repo,
                    kind=str(artifact.get("kind")),
                    artifact_id=str(artifact_id),
                    path=artifact_path,
                    evidence_commit=evidence_commit,
                )
                for key in (
                    "bbb_commit",
                    "bbb_provenance",
                    "capture_contract",
                    "evidence_class",
                    "exists",
                    "invalid_reasons",
                    "kind",
                    "scenario_ids",
                    "scoped_reuse",
                    "sha256",
                    "valid",
                ):
                    if artifact.get(key) != refreshed.get(key):
                        issues.append(
                            f"stale external artifact projection for {artifact_id}: {key}"
                        )
    for scenario_id, scenario in scenario_map.items():
        if scenario.get("mechanism_ids") != sorted(reverse_scenarios.get(str(scenario_id), set())):
            issues.append(f"stale reverse Mechanism-ID mapping for scenario {scenario_id}")
        if scenario.get("artifact_ids") != sorted(
            reverse_scenario_artifacts.get(str(scenario_id), set())
        ):
            issues.append(f"stale reverse artifact mapping for scenario {scenario_id}")
        kinds = reverse_scenario_kinds.get(str(scenario_id), set())
        statuses = reverse_scenario_statuses.get(str(scenario_id), set())
        if len(kinds) != 1 or scenario.get("kind") != next(iter(kinds), None):
            issues.append(f"stale kind mapping for scenario {scenario_id}")
        if len(statuses) != 1 or scenario.get("status") != next(iter(statuses), None):
            issues.append(f"stale status mapping for scenario {scenario_id}")
        if scenario.get("status") == "passed":
            for artifact_id in scenario.get("artifact_ids", []):
                artifact = artifact_map.get(artifact_id)
                if (
                    isinstance(artifact, dict)
                    and artifact.get("kind") not in {"repo-evidence", "gate"}
                    and artifact.get("evidence_class") != "formal"
                ):
                    issues.append(
                        f"diagnostic artifact used by passing scenario {scenario_id}"
                    )
    for item in report.get("generator_inputs", []):
        if not isinstance(item, dict) or not isinstance(item.get("path"), str):
            issues.append("malformed generator input")
            continue
        path = repo / item["path"]
        if not path.is_file() or sha256_file(path) != item.get("sha256"):
            issues.append(f"stale generator input: {item['path']}")
    source_hashes = {
        (source.get("path"), source.get("sha256"))
        for record in records
        if isinstance(record, dict)
        for source in record.get("vanilla", {}).get("source_hashes", [])
        if isinstance(source, dict)
    }
    if enforce_current:
        for relative, digest in sorted(source_hashes):
            if not isinstance(relative, str) or not isinstance(digest, str):
                issues.append("malformed Vanilla source hash")
                continue
            path = source_root / relative
            if not path.is_file() or sha256_file(path) != digest:
                issues.append(f"stale Vanilla source hash: {relative}")
    summary = report.get("summary")
    if isinstance(summary, dict):
        false_equivalents = sorted(
            record["mechanism_id"]
            for record in records
            if isinstance(record, dict)
            and record.get("matrix", {}).get("status") == "equivalent"
            and record.get("valid") is not True
        )
        if summary.get("false_equivalent_ids") != false_equivalents:
            issues.append("stale false-equivalent summary")
        invalid_ids = sorted(
            str(record["mechanism_id"])
            for record in records
            if isinstance(record, dict) and record.get("valid") is not True
        )
        if summary.get("declared_scope_complete") != (not invalid_ids):
            issues.append("stale declared-scope completion summary")
        if summary.get("invalid_ids") != invalid_ids:
            issues.append("stale invalid-mechanism summary")
        status_counts: dict[str, int] = defaultdict(int)
        for record in records:
            if isinstance(record, dict):
                status_counts[str(record.get("matrix", {}).get("status"))] += 1
        if summary.get("status_counts") != dict(sorted(status_counts.items())):
            issues.append("stale mechanism-status summary")
        if summary.get("mechanism_count") != len(records):
            issues.append("stale mechanism-count summary")
        if summary.get("valid_count") != len(records) - len(invalid_ids):
            issues.append("stale valid-mechanism summary")
    else:
        issues.append("report summary is malformed")
    if require_complete and isinstance(summary, dict) and summary.get("declared_scope_complete") is not True:
        issues.append("declared-scope parity acceptance is incomplete")
    expected_impact = change_impact_plan(
        [record for record in records if isinstance(record, dict)],
        changed_paths,
        evidence_commit,
        [artifact for artifact in artifacts if isinstance(artifact, dict)],
    )
    if report.get("change_impact") != expected_impact:
        issues.append("stale changed-path impact plan")
    affected_by_change = set(expected_impact["functional_affected_mechanism_ids"])
    for record in records:
        if (
            isinstance(record, dict)
            and record.get("mechanism_id") in affected_by_change
            and record.get("matrix", {}).get("status") == "equivalent"
            and "changed-path plan requires current validation"
            not in record.get("validity_reasons", [])
        ):
            issues.append(
                f"ungated central/dependency change for {record['mechanism_id']}"
            )

    consistency_rows = [
        row
        for row in rows
        if not row["mechanism_id"].startswith("performance-")
        and set(validation_classes(row))
        & {
            "protocol-state-receipt",
            "resource-platform-receipt",
            "visual-workload",
            "draw-evidence",
        }
    ]
    full_gate = artifact_map.get("full-gate")
    consistency_gate_passed = (
        isinstance(full_gate, dict)
        and full_gate.get("valid") is True
        and all(
            row["status"] in {"equivalent", "not-needed", "deferred-external"}
            and all(
                isinstance(artifact_map.get(artifact_id), dict)
                and artifact_map[artifact_id].get("valid") is True
                for artifact_id in external_artifact_ids(row)
                if artifact_id != "performance-abba"
            )
            for row in consistency_rows
        )
    )
    head, clean = current_head_and_clean(repo, evidence_commit)
    blocking_ids = implementation_blocking_ids(
        record for record in records if isinstance(record, dict)
    )
    implementation_ready = not blocking_ids
    code_freeze_ready = (
        implementation_ready
        and isinstance(full_gate, dict)
        and full_gate.get("valid") is True
        and not expected_impact["functional_changed_paths"]
        and not expected_impact["functional_affected_mechanism_ids"]
        and not expected_impact["validation_only_changed_paths"]
    )
    formal_performance_allowed = consistency_gate_passed and workflow_stage_index(
        str(stage)
    ) >= workflow_stage_index("final-performance")
    if workflow.get("consistency_gate_passed") is not consistency_gate_passed:
        issues.append("stale consistency-gate workflow result")
    if workflow.get("implementation_blocking_ids") != blocking_ids:
        issues.append("stale implementation-blocker workflow result")
    if workflow.get("implementation_ready") is not implementation_ready:
        issues.append("stale implementation-readiness workflow result")
    if workflow.get("code_freeze_ready") is not code_freeze_ready:
        issues.append("stale code-freeze workflow result")
    if workflow.get("formal_performance_allowed") is not formal_performance_allowed:
        issues.append("stale formal-performance workflow result")
    semantic_contract_sha256 = parity_semantic_contract_sha256(
        record for record in records if isinstance(record, dict)
    )
    if workflow.get("semantic_contract_sha256") != semantic_contract_sha256:
        issues.append("stale parity semantic-contract hash")
    if (
        workflow_stage_index(str(stage)) >= workflow_stage_index("code-freeze")
        and not code_freeze_ready
    ):
        issues.append("workflow stage advanced before a clean gated code freeze")
    if (
        workflow_stage_index(str(stage))
        >= workflow_stage_index("final-performance")
        and not consistency_gate_passed
    ):
        issues.append("workflow stage advanced to performance before consistency gates")
    performance_artifact = artifact_map.get("performance-abba")
    if (
        any(row["mechanism_id"].startswith("performance-") for row in rows)
        and workflow_stage_index(str(stage))
        >= workflow_stage_index("final-performance")
    ):
        capture_contract = (
            performance_artifact.get("capture_contract")
            if isinstance(performance_artifact, dict)
            else None
        )
        selected_contract = (
            capture_contract.get("selected_contract")
            if isinstance(capture_contract, dict)
            else None
        )
        prerequisites = (
            selected_contract.get("prerequisites")
            if isinstance(selected_contract, dict)
            else None
        )
        consistency_receipt = (
            prerequisites.get("consistency_receipt")
            if isinstance(prerequisites, dict)
            else None
        )
        if not isinstance(consistency_receipt, dict) or consistency_receipt.get(
            "semantic_contract_sha256"
        ) != workflow.get("semantic_contract_sha256"):
            issues.append(
                "final performance semantic contract differs from its consistency receipt"
            )

    scoped_formal_artifact_ids = sorted(
        str(artifact.get("artifact_id"))
        for artifact in artifacts
        if isinstance(artifact, dict)
        and artifact.get("evidence_class") == "formal"
        and artifact.get("scoped_reuse") is not None
    )
    projection_only_artifact_ids = sorted(
        str(artifact.get("artifact_id"))
        for artifact in artifacts
        if isinstance(artifact, dict)
        and artifact.get("evidence_class") == "formal"
        and artifact.get("projection_reuse") is not None
    )
    if (
        workflow_stage_index(str(stage))
        >= workflow_stage_index("evidence-sweep")
        and projection_only_artifact_ids
    ):
        issues.append(
            "evidence-sweep stage requires explicitly supplied formal artifacts; "
            "projection-only reuse remains: "
            + ", ".join(projection_only_artifact_ids)
        )
    scoped_consistency_artifact_ids = [
        artifact_id
        for artifact_id in scoped_formal_artifact_ids
        if artifact_id != "performance-abba"
    ]
    if (
        workflow_stage_index(str(stage))
        >= workflow_stage_index("final-performance")
        and scoped_consistency_artifact_ids
    ):
        issues.append(
            "final-performance stage requires exact-commit consistency receipts; "
            "scoped reuse remains: "
            + ", ".join(scoped_consistency_artifact_ids)
        )
    if stage == "final" and scoped_formal_artifact_ids:
        issues.append(
            "final workflow stage requires exact-commit formal receipts; "
            "scoped reuse remains: " + ", ".join(scoped_formal_artifact_ids)
        )

    if require_final_head:
        if stage != "final":
            issues.append("final exact-HEAD check requires workflow stage final")
        if not clean:
            issues.append("final exact-HEAD check requires a clean worktree")
        last_gated = repo / ".git/bbb/last-gated"
        gated_head = last_gated.read_text(encoding="utf-8").strip() if last_gated.is_file() else ""
        if gated_head != head:
            issues.append("final exact-HEAD check requires current HEAD to be full-gated")
        final_gate_path = None
        if isinstance(full_gate, dict) and isinstance(full_gate.get("path"), str):
            final_gate_path = Path(full_gate["path"])
            if not final_gate_path.is_absolute():
                final_gate_path = repo / final_gate_path
        if final_gate_path is None or gate_receipt(
            repo, final_gate_path, head
        ).get("valid") is not True:
            issues.append(
                "final exact-HEAD check requires a clean full-gate log receipt for current HEAD"
            )
        if expected_impact["functional_changed_paths"]:
            issues.append(
                "final exact-HEAD check permits only closure-metadata changes after evidence commit"
            )
        if not isinstance(summary, dict) or summary.get("declared_scope_complete") is not True:
            issues.append("final exact-HEAD check requires complete parity acceptance")
        if workflow.get("consistency_gate_passed") is not True:
            issues.append("final exact-HEAD check requires consistency gates to pass")
    return issues


def committed_report_artifacts(
    repo: Path, output: Path
) -> tuple[dict[str, dict[str, object]], dict[str, str]]:
    """Load reusable external projections from the committed canonical report."""

    expected = (repo / REPORT_PATH).resolve()
    if output.resolve() != expected:
        raise AcceptanceError(
            "--reuse-report-artifacts requires the canonical parity acceptance output"
        )
    commit = git(repo, "rev-parse", "HEAD")
    relative = REPORT_PATH.as_posix()
    show = subprocess.run(
        ["git", "show", f"{commit}:{relative}"],
        cwd=repo,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if show.returncode != 0:
        raise AcceptanceError("committed parity acceptance report is unavailable")
    try:
        report = json.loads(show.stdout.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise AcceptanceError(f"committed parity acceptance report is malformed: {error}") from error
    values = report.get("artifacts") if isinstance(report, dict) else None
    if not isinstance(values, list):
        raise AcceptanceError("committed parity acceptance report lacks artifacts")
    artifacts = {
        str(item["artifact_id"]): item
        for item in values
        if isinstance(item, dict) and isinstance(item.get("artifact_id"), str)
    }
    blob = git(repo, "rev-parse", f"{commit}:{relative}")
    return artifacts, {
        "source_report_blob_sha1": blob,
        "source_report_commit": commit,
        "source_report_path": relative,
        "source_report_sha256": hashlib.sha256(show.stdout).hexdigest(),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("generate", "check", "plan"))
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=REPO_ROOT / REPORT_PATH)
    parser.add_argument("--evidence-commit")
    parser.add_argument("--base-commit")
    parser.add_argument("--changed-path", action="append", default=[])
    parser.add_argument("--workflow-stage", choices=WORKFLOW_STAGES, default="implementation")
    parser.add_argument("--gate-log", type=Path)
    parser.add_argument("--performance-summary", type=Path)
    parser.add_argument("--workload-summary", type=Path)
    parser.add_argument("--draw-summary", type=Path)
    parser.add_argument("--platform-report", type=Path)
    parser.add_argument("--lifecycle-report", type=Path)
    parser.add_argument("--audio-report", type=Path)
    parser.add_argument(
        "--reuse-report-artifacts",
        action="store_true",
        help=(
            "reuse missing external artifact projections from the committed "
            "canonical report; final --require-artifacts still requires every file"
        ),
    )
    parser.add_argument("--allow-false-equivalent", action="store_true")
    parser.add_argument("--require-artifacts", action="store_true")
    parser.add_argument("--require-complete", action="store_true")
    parser.add_argument("--require-final-head", action="store_true")
    args = parser.parse_args()
    args.repo_root = args.repo_root.resolve()
    args.source_root = None if args.command == "plan" else resolve_source_root(args)
    if args.gate_log is None:
        args.gate_log = args.repo_root / GATE_LOG_PATH
    return args


def main() -> int:
    args = parse_args()
    if args.command == "generate":
        if args.require_final_head:
            print(
                "parity acceptance: --require-final-head is a post-commit check; generate, seal, full-gate, then run check",
                file=sys.stderr,
            )
            return 2
        assert isinstance(args.source_root, Path)
        evidence_commit = args.evidence_commit or git(args.repo_root, "rev-parse", "HEAD")
        reused_external_artifacts = None
        projection_source = None
        if args.reuse_report_artifacts:
            try:
                reused_external_artifacts, projection_source = committed_report_artifacts(
                    args.repo_root, args.output
                )
            except AcceptanceError as error:
                print(f"parity acceptance: {error}", file=sys.stderr)
                return 2
        report = build_report(
            repo=args.repo_root,
            source_root=args.source_root,
            evidence_commit=evidence_commit,
            gate_log=args.gate_log,
            workflow_stage=args.workflow_stage,
            performance_summary=args.performance_summary,
            workload_summary=args.workload_summary,
            draw_summary=args.draw_summary,
            platform_report=args.platform_report,
            lifecycle_report=args.lifecycle_report,
            audio_report=args.audio_report,
            reused_external_artifacts=reused_external_artifacts,
            projection_source=projection_source,
        )
        issues = validate_report(
            report,
            repo=args.repo_root,
            source_root=args.source_root,
            enforce_current=True,
            require_artifacts=args.require_artifacts,
            require_complete=args.require_complete,
            require_final_head=args.require_final_head,
        )
        false_equivalents = report["summary"]["false_equivalent_ids"]
        if false_equivalents and not args.allow_false_equivalent:
            issues.append("false equivalent rows: " + ", ".join(false_equivalents))
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(
            json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(
            f"parity acceptance: wrote {args.output} with "
            f"{report['summary']['mechanism_count']} mechanisms, "
            f"{len(false_equivalents)} false equivalents"
        )
        if issues and not args.allow_false_equivalent:
            for issue in sorted(set(issues)):
                print(f"parity acceptance: {issue}", file=sys.stderr)
            return 1
        return 0
    if not args.output.is_file():
        print(f"parity acceptance: report missing: {args.output}", file=sys.stderr)
        return 1
    report = json.loads(args.output.read_text(encoding="utf-8"))
    if args.command == "plan":
        records = report.get("mechanisms")
        if not isinstance(records, list):
            print("parity acceptance: report mechanisms are malformed", file=sys.stderr)
            return 1
        base_commit = args.base_commit or report.get("bbb_evidence_commit")
        if not isinstance(base_commit, str) or re.fullmatch(r"[0-9a-f]{40}", base_commit) is None:
            print("parity acceptance: plan requires an exact base commit", file=sys.stderr)
            return 1
        paths = set(args.changed_path) if args.changed_path else changed_repo_paths(
            args.repo_root, base_commit
        )
        paths.discard(REPORT_PATH.as_posix())
        report_artifacts = report.get("artifacts", [])
        planning_artifacts = artifacts_with_current_repo_inputs(
            args.repo_root,
            [
                artifact
                for artifact in report_artifacts
                if isinstance(artifact, dict)
            ]
            if isinstance(report_artifacts, list)
            else [],
        )
        print(
            json.dumps(
                change_impact_plan(
                    [record for record in records if isinstance(record, dict)],
                    paths,
                    base_commit,
                    planning_artifacts,
                ),
                ensure_ascii=False,
                indent=2,
                sort_keys=True,
            )
        )
        return 0
    assert isinstance(args.source_root, Path)
    issues = validate_report(
        report,
        repo=args.repo_root,
        source_root=args.source_root,
        enforce_current=True,
        require_artifacts=args.require_artifacts,
        require_complete=args.require_complete,
        require_final_head=args.require_final_head,
    )
    if issues:
        for issue in sorted(set(issues)):
            print(f"parity acceptance: {issue}", file=sys.stderr)
        return 1
    print(
        f"parity acceptance: {len(report['mechanisms'])} mechanisms, "
        "artifact/scenario reverse mappings and source hashes valid"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
