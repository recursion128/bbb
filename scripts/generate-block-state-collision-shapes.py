#!/usr/bin/env python3
"""Generate exact Vanilla 26.1 static block-state collision shapes."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DOWNLOAD_ROOT = Path.home() / "Work/mc-code/downloads/26.1"
DEFAULT_JDK_ROOT = Path.home() / ".local/jdks/jdk-25.0.3+9"
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/block_state_types/collision_shapes_26_1.rs"
)
BLOCK_STATES = REPO_ROOT / "crates/bbb-protocol/data/block_states_26_1.json"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockStateCollisionShapeReport.java"
DYNAMIC_SHAPE_INDEX = 0xFFFF

BoxBits = tuple[int, int, int, int, int, int]
ShapeBits = tuple[BoxBits, ...]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--client-jar",
        type=Path,
        default=DEFAULT_DOWNLOAD_ROOT / "client-26.1.jar",
    )
    parser.add_argument(
        "--libraries-root",
        type=Path,
        default=DEFAULT_DOWNLOAD_ROOT / "libraries",
    )
    parser.add_argument("--javac", type=Path, default=DEFAULT_JDK_ROOT / "bin/javac")
    parser.add_argument("--java", type=Path, default=DEFAULT_JDK_ROOT / "bin/java")
    parser.add_argument("--facts-input", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


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
    with tempfile.TemporaryDirectory(prefix="bbb-block-state-collision-") as directory:
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
                "BlockStateCollisionShapeReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla block-state collision probe failed:\n"
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
            f"expected {expected_count} runtime collision facts, got {len(lines) - 1}"
        )

    shapes: list[ShapeBits] = []
    dynamic: list[bool] = []
    for expected_id, line in enumerate(lines[1:]):
        fields = line.split("\t")
        if len(fields) < 3 or fields[0] != str(expected_id):
            raise ValueError(
                f"invalid runtime collision fact at id {expected_id}: {line!r}"
            )
        if fields[1] not in ("0", "1"):
            raise ValueError(
                f"invalid dynamic-shape fact at id {expected_id}: {fields[1]!r}"
            )
        try:
            box_count = int(fields[2])
        except ValueError as error:
            raise ValueError(
                f"invalid collision box count at id {expected_id}: {fields[2]!r}"
            ) from error
        if box_count < 0 or len(fields) != 3 + box_count * 6:
            raise ValueError(
                f"invalid collision box fields at id {expected_id}: {line!r}"
            )
        coordinates: list[int] = []
        for value in fields[3:]:
            try:
                bits = int(value)
            except ValueError as error:
                raise ValueError(
                    f"invalid collision coordinate bits at id {expected_id}: {value!r}"
                ) from error
            if not 0 <= bits <= 0xFFFF_FFFF_FFFF_FFFF:
                raise ValueError(
                    f"collision coordinate outside u64 at id {expected_id}: {bits}"
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


def render_rust(
    shapes: tuple[ShapeBits, ...], dynamic: tuple[bool, ...], probe_sha256: str
) -> str:
    if len(shapes) != len(dynamic):
        raise ValueError("collision shape/dynamic state counts differ")

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
                raise ValueError("collision shape palette does not fit in u16")
            shape_indices[shape] = index
            shape_palette.append(shape)
        state_indices.append(index)

    box_palette: list[BoxBits] = []
    shape_ranges: list[tuple[int, int]] = []
    for shape in shape_palette:
        start = len(box_palette)
        if start > 0xFFFF_FFFF:
            raise ValueError("collision box palette offset does not fit in u32")
        if len(shape) > 0xFF:
            raise ValueError("collision shape box count does not fit in u8")
        box_palette.extend(shape)
        shape_ranges.append((start, len(shape)))

    lines = [
        "// @generated by scripts/generate-block-state-collision-shapes.py; do not edit manually.",
        "// Source: Vanilla 26.1 BlockState.getCollisionShape(EmptyBlockGetter, BlockPos.ZERO).toAabbs().",
        "// Non-dynamic states are exact cached shapes; u16::MAX marks live-world dynamic shapes.",
        f"// Probe SHA-256: {probe_sha256}",
        "// Update: scripts/generate-block-state-collision-shapes.py",
        "// Check: scripts/generate-block-state-collision-shapes.py --check",
        "",
        "#[rustfmt::skip]",
        "pub(super) const VANILLA_BLOCK_COLLISION_BOX_PALETTE: &[[u64; 6]] = &[",
    ]
    lines.extend(
        "    [" + ", ".join(f"0x{value:016x}" for value in shape_box) + "],"
        for shape_box in box_palette
    )
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_COLLISION_SHAPE_RANGES: &[(u32, u8)] = &[",
        )
    )
    lines.extend(f"    ({start}, {count})," for start, count in shape_ranges)
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_COLLISION_SHAPE_INDICES: &[u16] = &[",
        )
    )
    for start in range(0, len(state_indices), 16):
        row = state_indices[start : start + 16]
        lines.append("    " + ", ".join(str(index) for index in row) + ",")
    lines.extend(("];", ""))
    return "\n".join(lines)


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


def main() -> int:
    args = parse_args()
    count = expected_state_count()
    facts = (
        args.facts_input.read_text()
        if args.facts_input is not None
        else runtime_facts(args)
    )
    shapes, dynamic = parse_facts(facts, count)
    generated = render_rust(
        shapes,
        dynamic,
        hashlib.sha256(PROBE_SOURCE.read_bytes()).hexdigest(),
    )
    if args.check:
        return 0 if check_output(args.output, generated) else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
