#!/usr/bin/env python3
"""Generate the bounded Vanilla 26.1 Overworld LIGHT component oracle."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_runtime, resolve_runtime_inputs

REPO_ROOT = Path(__file__).resolve().parents[1]
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/NoiseOverworldFeaturesWindowReport.java"
PROBE_CLASS = "NoiseOverworldFeaturesWindowReport"
DEFAULT_OUTPUT = REPO_ROOT / "scripts/data/noise-overworld-light-26.1.json"
SHA = re.compile(r"[0-9a-f]{64}\Z")
BITS = re.compile(r"[01]{26}\Z")
HEADER = (
    "version=26.1 source=controlled:all-air-init stage=light-component schema=1 seed=0 "
    "center=0,0 dependencies=9 dependency_order=x-major-z-minor "
    "init_orders=forward,reverse structure_topology=disabled "
    "controlled_emitter=glowstone@8,200,8 "
    "status_claim=component-only min_light_section=-5 light_sections=26 "
    "projection=clientbound-light-update"
)
MIN_LIGHT_SECTION = -5
LIGHT_SECTIONS = 26


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_runtime(parser)
    parser.add_argument("--report-input", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    resolve_runtime_inputs(args)
    return args


def runtime_report(args: argparse.Namespace) -> str:
    jars = [args.client_jar, *sorted(args.libraries_root.rglob("*.jar"))]
    classpath = os.pathsep.join(str(path) for path in jars)
    with tempfile.TemporaryDirectory(prefix="bbb-noise-overworld-light-") as directory:
        subprocess.run(
            [str(args.javac), "-cp", classpath, "-d", directory, str(PROBE_SOURCE)],
            check=True,
        )
        result = subprocess.run(
            [
                str(args.java),
                "-Xmx3G",
                "-cp",
                os.pathsep.join((directory, classpath)),
                PROBE_CLASS,
                "--light",
            ],
            check=False,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla Overworld LIGHT probe failed:\n"
                + result.stderr.strip()
                + "\nstdout:\n"
                + result.stdout.strip()
            )
    return result.stdout


def _sha(raw: str, name: str) -> str:
    if not SHA.fullmatch(raw):
        raise ValueError(f"{name} must be a lowercase SHA-256")
    return raw


def _count(raw: str, name: str) -> int:
    try:
        value = int(raw)
    except ValueError as error:
        raise ValueError(f"{name} must be an integer") from error
    if value < 0:
        raise ValueError(f"{name} must be nonnegative")
    return value


def _digest_rows(rows: list[str]) -> str:
    digest = hashlib.sha256()
    for row in rows:
        digest.update(row.encode())
        digest.update(b"\n")
    return digest.hexdigest()


def _set_indices(bits: str) -> list[int]:
    return [index for index, value in enumerate(bits) if value == "1"]


def parse_report(report: str) -> dict[str, object]:
    lines = [line for line in report.splitlines() if line]
    if len(lines) < 4 or lines[0] != HEADER:
        raise ValueError("unexpected Overworld LIGHT report header")

    projection = lines[1].split("\t")
    if len(projection) != 7 or projection[0] != "P":
        raise ValueError("malformed LIGHT projection row")
    sky_mask, block_mask, empty_sky_mask, empty_block_mask = projection[1:5]
    for name, bits in (
        ("sky mask", sky_mask),
        ("block mask", block_mask),
        ("empty sky mask", empty_sky_mask),
        ("empty block mask", empty_block_mask),
    ):
        if not BITS.fullmatch(bits):
            raise ValueError(f"{name} must contain exactly 26 bits")
    if any(
        value == "1" and empty == "1"
        for value, empty in zip(sky_mask, empty_sky_mask, strict=True)
    ):
        raise ValueError("sky and empty-sky masks overlap")
    if any(
        value == "1" and empty == "1"
        for value, empty in zip(block_mask, empty_block_mask, strict=True)
    ):
        raise ValueError("block and empty-block masks overlap")

    sky_count = _count(projection[5], "sky update count")
    block_count = _count(projection[6], "block update count")
    if sky_count != sky_mask.count("1") or block_count != block_mask.count("1"):
        raise ValueError("mask and update cardinality disagree")
    if sky_count == 0 or block_count == 0:
        raise ValueError("LIGHT corpus must exercise both sky and block arrays")
    expected_rows = 1 + sky_count + block_count
    if len(lines) != 1 + expected_rows + 1:
        raise ValueError("unexpected Overworld LIGHT report row count")

    updates = {"sky": [], "block": []}
    expected_indices = {
        "S": iter(_set_indices(sky_mask)),
        "B": iter(_set_indices(block_mask)),
    }
    prior_kind = "S"
    for line in lines[2:-1]:
        fields = line.split("\t")
        if len(fields) != 4 or fields[0] != "L" or fields[1] not in ("S", "B"):
            raise ValueError("malformed LIGHT layer row")
        kind = fields[1]
        if prior_kind == "B" and kind == "S":
            raise ValueError("sky layer rows must precede block layer rows")
        prior_kind = kind
        section_index = _count(fields[2], "light section index")
        try:
            expected_index = next(expected_indices[kind])
        except StopIteration as error:
            raise ValueError("layer rows exceed the corresponding mask") from error
        if section_index != expected_index:
            raise ValueError("layer row order does not match its mask")
        updates["sky" if kind == "S" else "block"].append(
            {
                "section_index": section_index,
                "section_y": MIN_LIGHT_SECTION + section_index,
                "bytes": 2048,
                "sha256": _sha(fields[3], "light layer digest"),
            }
        )

    summary = lines[-1].split("\t")
    if len(summary) != 3 or summary[0] != "A":
        raise ValueError("malformed LIGHT aggregate row")
    projection_digest = _digest_rows(lines[1:-1])
    forward = _sha(summary[1], "forward projection digest")
    reverse = _sha(summary[2], "reverse projection digest")
    if forward != projection_digest or reverse != projection_digest:
        raise ValueError("INIT order projections are not identical to the emitted rows")

    return {
        "artifact_kind": "vanilla-noise-overworld-light-component",
        "schema_version": 1,
        "vanilla_version": "26.1",
        "source_settings": "controlled:all-air-init",
        "seed": 0,
        "center": [0, 0],
        "input": {
            "stage": "initialize_light",
            "dependencies": 9,
            "dependency_order": "x-major-z-minor",
            "structure_topology": "disabled",
            "base_columns": "all-air",
            "controlled_emitter": {
                "block": "minecraft:glowstone",
                "position": [8, 200, 8],
            },
        },
        "scope": {
            "status_claim": "component-only",
            "min_light_section": MIN_LIGHT_SECTION,
            "light_sections": LIGHT_SECTIONS,
            "projection": "clientbound-light-update",
            "certifies": "controlled-center-block-then-sky-arrays",
            "does_not_certify": [
                "natural-features-input",
                "threaded-scheduling",
                "region-save-resume",
                "live-relight",
                "changed-section-filters",
                "transport",
            ],
        },
        "probe": {
            "path": "scripts/vanilla-runtime/NoiseOverworldFeaturesWindowReport.java",
            "sha256": hashlib.sha256(
                PROBE_SOURCE.read_text(encoding="utf-8").encode("utf-8")
            ).hexdigest(),
        },
        "projection": {
            "sky_mask": sky_mask,
            "block_mask": block_mask,
            "empty_sky_mask": empty_sky_mask,
            "empty_block_mask": empty_block_mask,
            "sky_updates": updates["sky"],
            "block_updates": updates["block"],
            "sha256": projection_digest,
        },
        "invariance": {
            "init_orders": ["forward", "reverse"],
            "projection_sha256": projection_digest,
        },
    }


def encode(artifact: dict[str, object]) -> str:
    return json.dumps(artifact, indent=2, sort_keys=True) + "\n"


def main() -> int:
    args = parse_args()
    report = (
        args.report_input.read_text(encoding="utf-8")
        if args.report_input
        else runtime_report(args)
    )
    expected = encode(parse_report(report))
    if args.check:
        actual = args.output.read_text(encoding="utf-8") if args.output.is_file() else ""
        if actual != expected:
            print(
                "".join(
                    difflib.unified_diff(
                        actual.splitlines(True),
                        expected.splitlines(True),
                        fromfile=str(args.output),
                        tofile="generated",
                    )
                )
            )
            return 1
        return 0
    args.output.write_text(expected, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
