#!/usr/bin/env python3
"""Generate Vanilla 26.1 destroy facts for every built-in block state."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_runtime, resolve_runtime_inputs

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/block_state_types/destroy_facts_26_1.rs"
)
DEFAULT_BIN_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/data/block_state_destroy_facts_26_1.bin"
)
BLOCK_STATES = REPO_ROOT / "crates/bbb-facts/data/block_states_26_1.json"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockStateDestroyFactsReport.java"


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
            "missing Vanilla runtime probe input: "
            + ", ".join(str(path) for path in missing)
        )
    classpath = os.pathsep.join(str(path) for path in jars)
    with tempfile.TemporaryDirectory(prefix="bbb-block-state-destroy-") as directory:
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
                "BlockStateDestroyFactsReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla block-state destroy probe failed:\n" + result.stderr.strip()
            )
    return result.stdout


def _coalesce(ids: list[int]) -> tuple[tuple[int, int], ...]:
    ranges: list[list[int]] = []
    for state_id in ids:
        if not ranges or state_id != ranges[-1][1] + 1:
            ranges.append([state_id, state_id])
        else:
            ranges[-1][1] = state_id
    return tuple((start, end) for start, end in ranges)


def parse_facts(
    source: str, expected_count: int
) -> tuple[list[int], tuple[tuple[int, int], ...]]:
    lines = source.splitlines()
    expected_header = f"version=26.1 count={expected_count}"
    if not lines or lines[0] != expected_header:
        raise ValueError(
            f"expected runtime report header {expected_header!r}, got "
            f"{lines[0] if lines else '<empty>'!r}"
        )
    if len(lines) != expected_count + 1:
        raise ValueError(
            f"expected {expected_count} runtime facts, got {len(lines) - 1}"
        )
    speed_bits: list[int] = []
    requires_correct_tool: list[int] = []
    for expected_id, line in enumerate(lines[1:]):
        fields = line.split("\t")
        if (
            len(fields) != 3
            or fields[0] != str(expected_id)
            or not fields[1].isdigit()
            or fields[2] not in ("0", "1")
        ):
            raise ValueError(f"invalid runtime fact at id {expected_id}: {line!r}")
        bits = int(fields[1])
        if not 0 <= bits <= 0xFFFF_FFFF:
            raise ValueError(f"destroy speed bits out of range at id {expected_id}")
        speed_bits.append(bits)
        if fields[2] == "1":
            requires_correct_tool.append(expected_id)
    return speed_bits, _coalesce(requires_correct_tool)


def build_palette(speed_bits: list[int]) -> tuple[list[int], list[int]]:
    """Distinct destroy speeds in first-appearance order, plus one index per state."""
    palette: list[int] = []
    index_of: dict[int, int] = {}
    indices: list[int] = []
    for bits in speed_bits:
        index = index_of.get(bits)
        if index is None:
            index = len(palette)
            index_of[bits] = index
            palette.append(bits)
        indices.append(index)
    if len(palette) > 0xFF:
        raise ValueError(
            f"destroy speed palette needs {len(palette)} entries, which no longer "
            "fits the u8 index table"
        )
    return palette, indices


def pack_binary(
    palette: list[int],
    indices: list[int],
    requires_correct_tool: tuple[tuple[int, int], ...],
) -> bytes:
    state_count = len(indices)
    bit_bytes = bytearray((state_count + 7) // 8)
    for start, end in requires_correct_tool:
        for state_id in range(start, end + 1):
            bit_bytes[state_id >> 3] |= 1 << (state_id & 7)
    out = bytearray()
    for bits in palette:
        out.extend(struct.pack("<I", bits))
    out.extend(indices)
    out.extend(bit_bytes)
    return bytes(out)


def render_rust(
    palette: list[int],
    indices: list[int],
    requires_correct_tool: tuple[tuple[int, int], ...],
    bin_rel_path: str = "../../data/block_state_destroy_facts_26_1.bin",
) -> str:
    _ = requires_correct_tool
    palette_len = len(palette)
    state_count = len(indices)
    indices_offset = palette_len * 4
    requires_offset = indices_offset + state_count
    lines = [
        "// @generated by scripts/generate-block-state-destroy-facts.py; do not edit manually.",
        "// Source: Vanilla 26.1 BlockState.getDestroySpeed(level, pos) and",
        "// BlockState.requiresCorrectToolForDrops(), evaluated by the checked-in runtime",
        "// probe. These are the two block-side inputs of BlockBehaviour.getDestroyProgress",
        "// (BlockBehaviour.java:355-363).",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-block-state-destroy-facts.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-block-state-destroy-facts.py --check",
        "",
        f'const BYTES: &[u8] = include_bytes!("{bin_rel_path}");',
        f"pub(super) const PALETTE_LEN: usize = {palette_len};",
        f"pub(super) const STATE_COUNT: usize = {state_count};",
        f"const INDICES_OFFSET: usize = {indices_offset};",
        f"const REQUIRES_OFFSET: usize = {requires_offset};",
        "",
        "#[inline]",
        "pub(super) fn speed_index(state_id: usize) -> Option<u8> {",
        "    if state_id >= STATE_COUNT {",
        "        return None;",
        "    }",
        "    Some(BYTES[INDICES_OFFSET + state_id])",
        "}",
        "",
        "#[inline]",
        "pub(super) fn destroy_speed_bits(state_id: usize) -> Option<u32> {",
        "    let palette_index = usize::from(speed_index(state_id)?);",
        "    if palette_index >= PALETTE_LEN {",
        "        return None;",
        "    }",
        "    let offset = palette_index * 4;",
        "    Some(u32::from_le_bytes([",
        "        BYTES[offset],",
        "        BYTES[offset + 1],",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "    ]))",
        "}",
        "",
        "#[inline]",
        "pub(super) fn requires_correct_tool(state_id: usize) -> Option<bool> {",
        "    if state_id >= STATE_COUNT {",
        "        return None;",
        "    }",
        "    Some(BYTES[REQUIRES_OFFSET + (state_id >> 3)] & (1 << (state_id & 7)) != 0)",
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
    facts = (
        args.facts_input.read_text()
        if args.facts_input is not None
        else runtime_facts(args)
    )
    speed_bits, requires_correct_tool = parse_facts(facts, count)
    palette, indices = build_palette(speed_bits)
    bin_output, bin_rel = _resolve_bin_output(args.output, args.bin_output)
    generated = render_rust(palette, indices, requires_correct_tool, bin_rel)
    blob = pack_binary(palette, indices, requires_correct_tool)
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
