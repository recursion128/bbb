#!/usr/bin/env python3
"""Generate exact Vanilla 26.1 ground-path facts for every built-in block state."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

from bbb_tools.vanilla.arguments import (
    add_vanilla_runtime,
    resolve_runtime_inputs,
    resolve_source_root,
)
from block_state_water_path_rules import (
    LIQUID_SOURCE_NOT_LAVA,
    STATE_FLUID_WATER,
    STATIC,
    classify_owner,
)

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/block_state_types/path_facts_26_1.rs"
DEFAULT_BIN_OUTPUT = REPO_ROOT / "crates/bbb-facts/data/block_state_path_facts_26_1.bin"
BLOCK_STATES = REPO_ROOT / "crates/bbb-facts/data/block_states_26_1.json"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockStatePathFactsReport.java"
PROBE_FACT_BITS = (1 << 11) - 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_runtime(parser)
    parser.add_argument("--facts-input", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--bin-output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    resolve_runtime_inputs(args)
    return args


def expected_state_count() -> int:
    report = json.loads(BLOCK_STATES.read_text())
    if report.get("version") != "26.1":
        raise ValueError("embedded block-state report is not for Vanilla 26.1")
    states = report.get("states")
    if not isinstance(states, list):
        raise ValueError("embedded block-state report lacks states")
    for expected_id, state in enumerate(states):
        if state.get("id") != expected_id:
            raise ValueError("embedded block-state report is not contiguous")
    return len(states)


def runtime_facts(args: argparse.Namespace) -> str:
    jars = [args.client_jar, *sorted(args.libraries_root.rglob("*.jar"))]
    missing = [path for path in (args.javac, args.java, *jars[:1]) if not path.is_file()]
    if missing:
        raise FileNotFoundError(
            "missing Vanilla runtime probe input: " + ", ".join(str(path) for path in missing)
        )
    classpath = os.pathsep.join(str(path) for path in jars)
    with tempfile.TemporaryDirectory(prefix="bbb-block-state-path-facts-") as directory:
        classes = Path(directory)
        subprocess.run(
            [
                str(args.javac),
                "-cp",
                classpath,
                "-d",
                str(classes),
                str(PROBE_SOURCE),
            ],
            check=True,
        )
        result = subprocess.run(
            [
                str(args.java),
                "-cp",
                os.pathsep.join((str(classes), classpath)),
                "BlockStatePathFactsReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla block-state path-facts probe failed:\n" + result.stderr.strip()
            )
    return result.stdout


def parse_facts(source: str, expected_count: int, source_root: Path) -> tuple[int, ...]:
    lines = source.splitlines()
    expected_header = f"version=26.1 count={expected_count}"
    if not lines or lines[0] != expected_header:
        raise ValueError(
            f"expected runtime report header {expected_header!r}, got "
            f"{lines[0] if lines else '<empty>'!r}"
        )
    if len(lines) != expected_count + 1:
        raise ValueError(f"expected {expected_count} runtime path facts, got {len(lines) - 1}")

    facts: list[int] = []
    rules: dict[str, int] = {}
    for expected_id, line in enumerate(lines[1:]):
        fields = line.split("\t")
        if len(fields) != 5 or fields[0] != str(expected_id) or not fields[1].isdigit():
            raise ValueError(f"invalid runtime path fact at id {expected_id}: {line!r}")
        bits = int(fields[1])
        if bits & ~PROBE_FACT_BITS:
            raise ValueError(f"unknown path-fact bits at id {expected_id}: {bits}")
        if bits & ((1 << 3) | (1 << 4)) and not bits & (1 << 2):
            raise ValueError(f"door detail without door identity at id {expected_id}")
        if bits & (1 << 8) and not bits & (1 << 7):
            raise ValueError(f"fence-gate open without gate identity at id {expected_id}")
        owner, source_fluid, state_fluid = fields[2:]
        if owner not in rules:
            rules[owner] = classify_owner(source_root, owner)
        rule = rules[owner]
        if rule not in {STATIC, STATE_FLUID_WATER, LIQUID_SOURCE_NOT_LAVA}:
            raise ValueError(f"reserved WATER path rule at id {expected_id}: {rule}")
        if state_fluid not in {
            "minecraft:empty",
            "minecraft:water",
            "minecraft:flowing_water",
            "minecraft:lava",
            "minecraft:flowing_lava",
        }:
            raise ValueError(f"unknown state fluid at id {expected_id}: {state_fluid}")
        if rule == LIQUID_SOURCE_NOT_LAVA:
            if source_fluid not in {"minecraft:water", "minecraft:lava"}:
                raise ValueError(f"unknown liquid source at id {expected_id}: {source_fluid}")
            expected_water = source_fluid != "minecraft:lava"
        else:
            if source_fluid != "-":
                raise ValueError(f"unexpected source fluid at id {expected_id}: {source_fluid}")
            expected_water = (
                state_fluid in {"minecraft:water", "minecraft:flowing_water"}
                if rule == STATE_FLUID_WATER
                else bool(bits & (1 << 10))
            )
        if expected_water != bool(bits & (1 << 10)):
            raise ValueError(f"WATER path rule disagrees with runtime at id {expected_id}")
        facts.append(bits | rule << 11)
    return tuple(facts)


def build_runs(facts: tuple[int, ...]) -> tuple[tuple[int, int], ...]:
    if not facts:
        return ()
    runs: list[tuple[int, int]] = []
    current = facts[0]
    for state_id, bits in enumerate(facts[1:], start=1):
        if bits != current:
            runs.append((state_id - 1, current))
            current = bits
    runs.append((len(facts) - 1, current))
    if runs[-1][0] > 0xFFFF:
        raise ValueError("block-state id does not fit in the u16 run table")
    return tuple(runs)


def pack_binary(runs: tuple[tuple[int, int], ...]) -> bytes:
    out = bytearray()
    for end, bits in runs:
        out.extend(struct.pack("<HH", end, bits))
    return bytes(out)


def render_rust(
    runs: tuple[tuple[int, int], ...],
    probe_sha256: str,
    bin_rel_path: str = "../../data/block_state_path_facts_26_1.bin",
) -> str:
    lines = [
        "// @generated by scripts/generate-block-state-path-facts.py; do not edit manually.",
        "// Source: Vanilla 26.1 BlockState.isPathfindable(LAND/WATER) "
        "(BlockBehaviour.java:136-146).",
        "// Ground path classifiers: WalkNodeEvaluator.java:514-581 and "
        "NodeEvaluator.java:98-104; lit campfire: CampfireBlock.java:278-280.",
        f"// Probe SHA-256: {probe_sha256}",
        "// Update: scripts/generate-block-state-path-facts.py",
        "// Check: scripts/generate-block-state-path-facts.py --check",
        "",
        f'const BYTES: &[u8] = include_bytes!("{bin_rel_path}");',
        f"pub(super) const RUN_COUNT: usize = {len(runs)};",
        "",
        "#[inline]",
        "pub(super) fn run_at(index: usize) -> (u16, u16) {",
        "    let offset = index * 4;",
        "    (",
        "        u16::from_le_bytes([BYTES[offset], BYTES[offset + 1]]),",
        "        u16::from_le_bytes([BYTES[offset + 2], BYTES[offset + 3]]),",
        "    )",
        "}",
        "",
        "pub(super) fn path_fact_bits(state_id: u16) -> Option<u16> {",
        "    let mut lo = 0usize;",
        "    let mut hi = RUN_COUNT;",
        "    while lo < hi {",
        "        let mid = lo + (hi - lo) / 2;",
        "        if run_at(mid).0 < state_id {",
        "            lo = mid + 1;",
        "        } else {",
        "            hi = mid;",
        "        }",
        "    }",
        "    (lo < RUN_COUNT).then(|| run_at(lo).1)",
        "}",
        "",
    ]
    return "\n".join(lines)


def _resolve_bin_output(output: Path, bin_output: Path | None) -> tuple[Path, str]:
    if bin_output is None:
        bin_output = (
            DEFAULT_BIN_OUTPUT
            if output == DEFAULT_OUTPUT
            else output.with_suffix(".bin")
        )
    rel = os.path.relpath(bin_output, output.parent).replace("\\", "/")
    return bin_output, rel


def check_output(path: Path, generated: str) -> bool:
    current = path.read_text() if path.exists() else ""
    if current == generated:
        return True
    sys.stderr.writelines(
        difflib.unified_diff(
            current.splitlines(keepends=True),
            generated.splitlines(keepends=True),
            fromfile=str(path),
            tofile="generated",
        )
    )
    return False


def check_bytes_output(path: Path, generated: bytes) -> bool:
    current = path.read_bytes() if path.exists() else b""
    if current == generated:
        return True
    sys.stderr.write(
        f"binary output differs at {path} (actual {len(current)} B vs expected {len(generated)} B)\n"
    )
    return False


def main() -> int:
    args = parse_args()
    count = expected_state_count()
    source = args.facts_input.read_text() if args.facts_input is not None else runtime_facts(args)
    facts = parse_facts(source, count, resolve_source_root(args))
    runs = build_runs(facts)
    bin_output, bin_rel = _resolve_bin_output(args.output, args.bin_output)
    generated = render_rust(
        runs,
        hashlib.sha256(PROBE_SOURCE.read_bytes()).hexdigest(),
        bin_rel,
    )
    blob = pack_binary(runs)
    if args.check:
        rust_ok = check_output(args.output, generated)
        bin_ok = check_bytes_output(bin_output, blob)
        return 0 if rust_ok and bin_ok else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    bin_output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    bin_output.write_bytes(blob)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
