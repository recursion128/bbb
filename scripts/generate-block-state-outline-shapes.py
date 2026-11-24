#!/usr/bin/env python3
"""Generate exact Vanilla 26.1 static block-state outline shapes."""

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

from bbb_tools.vanilla.arguments import add_vanilla_runtime, resolve_runtime_inputs

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/block_state_types/outline_shapes_26_1.rs"
)
DEFAULT_BIN_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/data/block_state_outline_shapes_26_1.bin"
)
BLOCK_STATES = REPO_ROOT / "crates/bbb-facts/data/block_states_26_1.json"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockStateOutlineShapeReport.java"
DYNAMIC_SHAPE_INDEX = 0xFFFF

BoxBits = tuple[int, int, int, int, int, int]
ShapeBits = tuple[BoxBits, ...]


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
    with tempfile.TemporaryDirectory(prefix="bbb-block-state-outline-") as directory:
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
                "BlockStateOutlineShapeReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla block-state outline probe failed:\n"
                + result.stderr.strip()
            )
    return result.stdout


def parse_facts(
    source: str, expected_count: int
) -> tuple[tuple[ShapeBits, ...], tuple[bool, ...]]:
    lines = source.splitlines()
    expected_header = f"version=26.1 count={expected_count}"
    if not lines or lines[0] != expected_header:
        raise ValueError(
            f"expected runtime report header {expected_header!r}, got "
            f"{lines[0] if lines else '<empty>'!r}"
        )
    if len(lines) != expected_count + 1:
        raise ValueError(
            f"expected {expected_count} runtime outline facts, got {len(lines) - 1}"
        )

    shapes: list[ShapeBits] = []
    dynamic: list[bool] = []
    for expected_id, line in enumerate(lines[1:]):
        fields = line.split("\t")
        if len(fields) < 3 or fields[0] != str(expected_id):
            raise ValueError(
                f"invalid runtime outline fact at id {expected_id}: {line!r}"
            )
        if fields[1] not in ("0", "1"):
            raise ValueError(
                f"invalid dynamic-shape fact at id {expected_id}: {fields[1]!r}"
            )
        try:
            box_count = int(fields[2])
        except ValueError as error:
            raise ValueError(
                f"invalid outline box count at id {expected_id}: {fields[2]!r}"
            ) from error
        if box_count < 0 or len(fields) != 3 + box_count * 6:
            raise ValueError(
                f"invalid outline box fields at id {expected_id}: {line!r}"
            )
        coordinates: list[int] = []
        for value in fields[3:]:
            try:
                bits = int(value)
            except ValueError as error:
                raise ValueError(
                    f"invalid outline coordinate bits at id {expected_id}: {value!r}"
                ) from error
            if not 0 <= bits <= 0xFFFF_FFFF_FFFF_FFFF:
                raise ValueError(
                    f"outline coordinate outside u64 at id {expected_id}: {bits}"
                )
            coordinates.append(bits)
        shapes.append(
            tuple(
                tuple(coordinates[start : start + 6])  # type: ignore[arg-type]
                for start in range(0, len(coordinates), 6)
            )
        )
        dynamic.append(fields[1] == "1")
    return tuple(shapes), tuple(dynamic)


def build_tables(
    shapes: tuple[ShapeBits, ...], dynamic: tuple[bool, ...]
) -> tuple[list[BoxBits], list[tuple[int, int]], list[int]]:
    if len(shapes) != len(dynamic):
        raise ValueError("outline shape/dynamic state counts differ")

    shape_palette: list[ShapeBits] = []
    shape_indices: dict[ShapeBits, int] = {}
    state_indices: list[int] = []
    for shape, is_dynamic in zip(shapes, dynamic, strict=True):
        if is_dynamic:
            state_indices.append(DYNAMIC_SHAPE_INDEX)
            continue
        index = shape_indices.get(shape)
        if index is None:
            index = len(shape_palette)
            if index >= DYNAMIC_SHAPE_INDEX:
                raise ValueError("outline shape palette does not fit in u16")
            shape_indices[shape] = index
            shape_palette.append(shape)
        state_indices.append(index)

    box_palette: list[BoxBits] = []
    shape_ranges: list[tuple[int, int]] = []
    for shape in shape_palette:
        start = len(box_palette)
        if start > 0xFFFF_FFFF:
            raise ValueError("outline box palette offset does not fit in u32")
        if len(shape) > 0xFF:
            raise ValueError("outline shape box count does not fit in u8")
        box_palette.extend(shape)
        shape_ranges.append((start, len(shape)))
    return box_palette, shape_ranges, state_indices


def pack_binary(
    shapes: tuple[ShapeBits, ...], dynamic: tuple[bool, ...]
) -> bytes:
    box_palette, shape_ranges, state_indices = build_tables(shapes, dynamic)
    out = bytearray()
    for index in state_indices:
        out.extend(struct.pack("<H", index))
    for start, count in shape_ranges:
        out.extend(struct.pack("<IB", start, count))
    for shape_box in box_palette:
        out.extend(struct.pack("<6Q", *shape_box))
    return bytes(out)


def render_rust(
    shapes: tuple[ShapeBits, ...],
    dynamic: tuple[bool, ...],
    probe_sha256: str,
    bin_rel_path: str = "../../data/block_state_outline_shapes_26_1.bin",
) -> str:
    box_palette, shape_ranges, state_indices = build_tables(shapes, dynamic)
    state_count = len(state_indices)
    range_count = len(shape_ranges)
    box_count = len(box_palette)
    ranges_offset = state_count * 2
    boxes_offset = ranges_offset + range_count * 5
    lines = [
        "// @generated by scripts/generate-block-state-outline-shapes.py; do not edit manually.",
        "// Source: Vanilla 26.1 BlockState.getShape(EmptyBlockGetter, BlockPos.ZERO).toAabbs().",
        "// Non-dynamic states are exact cached shapes; u16::MAX marks live-world dynamic shapes.",
        f"// Probe SHA-256: {probe_sha256}",
        "// Update: scripts/generate-block-state-outline-shapes.py",
        "// Check: scripts/generate-block-state-outline-shapes.py --check",
        "",
        f'const BYTES: &[u8] = include_bytes!("{bin_rel_path}");',
        f"pub(super) const STATE_COUNT: usize = {state_count};",
        f"pub(super) const RANGE_COUNT: usize = {range_count};",
        f"pub(super) const BOX_COUNT: usize = {box_count};",
        f"const RANGES_OFFSET: usize = {ranges_offset};",
        f"const BOXES_OFFSET: usize = {boxes_offset};",
        "const _: () = assert!(BYTES.len() == BOXES_OFFSET + BOX_COUNT * 48);",
        "",
        "#[inline]",
        "pub(super) fn shape_index(state_id: usize) -> Option<u16> {",
        "    if state_id >= STATE_COUNT {",
        "        return None;",
        "    }",
        "    let offset = state_id * 2;",
        "    Some(u16::from_le_bytes([BYTES[offset], BYTES[offset + 1]]))",
        "}",
        "",
        "#[inline]",
        "pub(super) fn shape_range(palette_index: usize) -> Option<(u32, u8)> {",
        "    if palette_index >= RANGE_COUNT {",
        "        return None;",
        "    }",
        "    let offset = RANGES_OFFSET + palette_index * 5;",
        "    let start = u32::from_le_bytes([",
        "        BYTES[offset],",
        "        BYTES[offset + 1],",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "    ]);",
        "    Some((start, BYTES[offset + 4]))",
        "}",
        "",
        "#[inline]",
        "pub(super) fn shape_box_bytes(state_id: usize) -> Option<&'static [u8]> {",
        "    let palette_index = shape_index(state_id)?;",
        "    if palette_index == u16::MAX {",
        "        return None;",
        "    }",
        "    let (start, count) = shape_range(usize::from(palette_index))?;",
        "    if (start as usize) + usize::from(count) > BOX_COUNT {",
        "        return None;",
        "    }",
        "    let byte_start = BOXES_OFFSET + (start as usize) * 48;",
        "    let byte_end = byte_start + usize::from(count) * 48;",
        "    BYTES.get(byte_start..byte_end)",
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
    shapes, dynamic = parse_facts(facts, count)
    bin_output, bin_rel = _resolve_bin_output(args.output, args.bin_output)
    generated = render_rust(
        shapes,
        dynamic,
        hashlib.sha256(PROBE_SOURCE.read_text().encode()).hexdigest(),
        bin_rel,
    )
    blob = pack_binary(shapes, dynamic)
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
