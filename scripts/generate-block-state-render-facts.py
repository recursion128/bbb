#!/usr/bin/env python3
"""Generate exact Vanilla 26.1 block-state renderer mechanism facts."""

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
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DOWNLOAD_ROOT = Path.home() / "Work/mc-code/downloads/26.1"
DEFAULT_JDK_ROOT = Path.home() / ".local/jdks/jdk-25.0.3+9"
DEFAULT_PROTOCOL_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/block_state_types/render_facts_26_1.rs"
)
DEFAULT_RENDERER_OUTPUT = (
    REPO_ROOT / "crates/bbb-renderer/src/terrain/face_occlusion_26_1.rs"
)
BLOCK_STATES = REPO_ROOT / "crates/bbb-protocol/data/block_states_26_1.json"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockStateRenderFactsReport.java"

FaceMask = tuple[int, int, int, int]


@dataclass(frozen=True)
class StateRenderFacts:
    block_type_id: int
    behavior_bits: int
    light_emission: int
    emissive_rendering: bool
    face_masks: tuple[FaceMask, FaceMask, FaceMask, FaceMask, FaceMask, FaceMask]


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
    parser.add_argument("--protocol-output", type=Path, default=DEFAULT_PROTOCOL_OUTPUT)
    parser.add_argument("--renderer-output", type=Path, default=DEFAULT_RENDERER_OUTPUT)
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
    with tempfile.TemporaryDirectory(prefix="bbb-block-state-render-") as directory:
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
                "BlockStateRenderFactsReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla block-state render probe failed:\n" + result.stderr.strip()
            )
    return result.stdout


def _double_from_unsigned_bits(value: str, state_id: int) -> float:
    try:
        bits = int(value)
    except ValueError as error:
        raise ValueError(
            f"invalid face coordinate bits at state id {state_id}: {value!r}"
        ) from error
    if not 0 <= bits <= 0xFFFF_FFFF_FFFF_FFFF:
        raise ValueError(f"face coordinate outside u64 at state id {state_id}: {bits}")
    return struct.unpack(">d", bits.to_bytes(8, "big"))[0]


def _grid_coordinate(value: float, state_id: int) -> int:
    scaled = value * 16.0
    rounded = round(scaled)
    if not 0 <= rounded <= 16 or abs(scaled - rounded) > 1.0e-9:
        raise ValueError(
            f"face coordinate is not on Vanilla's 1/16 grid at state id "
            f"{state_id}: {value!r}"
        )
    return rounded


def _face_mask(rectangles: list[tuple[float, float, float, float]], state_id: int) -> FaceMask:
    words = [0, 0, 0, 0]
    for min_a, min_b, max_a, max_b in rectangles:
        a0 = _grid_coordinate(min_a, state_id)
        b0 = _grid_coordinate(min_b, state_id)
        a1 = _grid_coordinate(max_a, state_id)
        b1 = _grid_coordinate(max_b, state_id)
        if a0 > a1 or b0 > b1:
            raise ValueError(f"inverted face rectangle at state id {state_id}")
        for b in range(b0, b1):
            for a in range(a0, a1):
                bit = b * 16 + a
                words[bit // 64] |= 1 << (bit % 64)
    return tuple(words)  # type: ignore[return-value]


def parse_facts(source: str, expected_count: int) -> tuple[StateRenderFacts, ...]:
    lines = source.splitlines()
    expected_header = f"version=26.1 count={expected_count}"
    if not lines or lines[0] != expected_header:
        raise ValueError(
            f"expected runtime report header {expected_header!r}, got "
            f"{lines[0] if lines else '<empty>'!r}"
        )
    if len(lines) != expected_count + 1:
        raise ValueError(
            f"expected {expected_count} runtime render facts, got {len(lines) - 1}"
        )

    result: list[StateRenderFacts] = []
    for expected_id, line in enumerate(lines[1:]):
        fields = line.split("\t")
        if len(fields) < 11 or fields[0] != str(expected_id):
            raise ValueError(f"invalid runtime render fact at id {expected_id}: {line!r}")
        try:
            block_type_id = int(fields[1])
            behavior_bits = int(fields[2])
            light_emission = int(fields[3])
            emissive_rendering_raw = int(fields[4])
        except ValueError as error:
            raise ValueError(
                f"invalid renderer identity facts at id {expected_id}: {line!r}"
            ) from error
        if not 0 <= block_type_id <= 0xFFFE:
            raise ValueError(f"block type id outside u16 at state id {expected_id}")
        if not 0 <= behavior_bits <= 0xFFFF:
            raise ValueError(f"render behavior bits outside u16 at state id {expected_id}")
        if not 0 <= light_emission <= 15:
            raise ValueError(f"light emission outside 0..=15 at state id {expected_id}")
        if emissive_rendering_raw not in (0, 1):
            raise ValueError(f"emissive rendering is not boolean at state id {expected_id}")

        cursor = 5
        face_masks: list[FaceMask] = []
        for _ in range(6):
            if cursor >= len(fields):
                raise ValueError(f"missing face shape at state id {expected_id}")
            try:
                rectangle_count = int(fields[cursor])
            except ValueError as error:
                raise ValueError(
                    f"invalid face rectangle count at state id {expected_id}"
                ) from error
            cursor += 1
            if rectangle_count < 0 or cursor + rectangle_count * 4 > len(fields):
                raise ValueError(f"invalid face rectangles at state id {expected_id}")
            rectangles = []
            for _ in range(rectangle_count):
                coordinates = tuple(
                    _double_from_unsigned_bits(value, expected_id)
                    for value in fields[cursor : cursor + 4]
                )
                cursor += 4
                rectangles.append(coordinates)
            face_masks.append(_face_mask(rectangles, expected_id))
        if cursor != len(fields):
            raise ValueError(f"trailing face shape fields at state id {expected_id}")
        result.append(
            StateRenderFacts(
                block_type_id=block_type_id,
                behavior_bits=behavior_bits,
                light_emission=light_emission,
                emissive_rendering=bool(emissive_rendering_raw),
                face_masks=tuple(face_masks),  # type: ignore[arg-type]
            )
        )
    return tuple(result)


def build_palette(
    facts: tuple[StateRenderFacts, ...],
) -> tuple[tuple[FaceMask, ...], tuple[tuple[int, int, int, int, int, int], ...]]:
    palette: list[FaceMask] = []
    indices: dict[FaceMask, int] = {}
    state_indices = []
    for state in facts:
        face_indices = []
        for mask in state.face_masks:
            index = indices.get(mask)
            if index is None:
                index = len(palette)
                if index > 0xFF:
                    raise ValueError("face occlusion palette does not fit in u8")
                indices[mask] = index
                palette.append(mask)
            face_indices.append(index)
        state_indices.append(tuple(face_indices))
    return tuple(palette), tuple(state_indices)  # type: ignore[return-value]


def render_protocol_rust(
    facts: tuple[StateRenderFacts, ...],
    state_indices: tuple[tuple[int, int, int, int, int, int], ...],
    probe_sha256: str,
) -> str:
    lines = [
        "// @generated by scripts/generate-block-state-render-facts.py; do not edit manually.",
        "// Source: Vanilla 26.1 block identity, light-coordinate state facts, skipRendering class semantics, FluidRenderer overlay relation, and BlockState.getFaceOcclusionShape().",
        f"// Probe SHA-256: {probe_sha256}",
        "// Update: scripts/generate-block-state-render-facts.py",
        "// Check: scripts/generate-block-state-render-facts.py --check",
        "",
        "#[rustfmt::skip]",
        "pub(super) const VANILLA_BLOCK_STATE_RENDER_BLOCK_IDS: &[u16] = &[",
    ]
    block_ids = [state.block_type_id for state in facts]
    for start in range(0, len(block_ids), 16):
        lines.append("    " + ", ".join(str(value) for value in block_ids[start : start + 16]) + ",")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_RENDER_BEHAVIOR_BITS: &[u16] = &[",
        )
    )
    behavior = [state.behavior_bits for state in facts]
    for start in range(0, len(behavior), 16):
        lines.append("    " + ", ".join(str(value) for value in behavior[start : start + 16]) + ",")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_LIGHT_EMISSION: &[u8] = &[",
        )
    )
    light_emission = [state.light_emission for state in facts]
    for start in range(0, len(light_emission), 32):
        lines.append("    " + ", ".join(str(value) for value in light_emission[start : start + 32]) + ",")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_EMISSIVE_RENDERING_BITS: &[u64] = &[",
        )
    )
    emissive_rendering = [state.emissive_rendering for state in facts]
    for start in range(0, len(emissive_rendering), 64):
        word = sum(
            1 << bit
            for bit, value in enumerate(emissive_rendering[start : start + 64])
            if value
        )
        lines.append(f"    0x{word:016x},")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_FACE_OCCLUSION_SHAPE_INDICES: &[[u8; 6]] = &[",
        )
    )
    lines.extend(
        "    [" + ", ".join(str(value) for value in indices) + "],"
        for indices in state_indices
    )
    lines.extend(("];", ""))
    return "\n".join(lines)


def render_renderer_rust(palette: tuple[FaceMask, ...], probe_sha256: str) -> str:
    lines = [
        "// @generated by scripts/generate-block-state-render-facts.py; do not edit manually.",
        "// Exact 16x16 face occupancy palette for Vanilla 26.1 BlockState.getFaceOcclusionShape().",
        f"// Probe SHA-256: {probe_sha256}",
        "// Update: scripts/generate-block-state-render-facts.py",
        "// Check: scripts/generate-block-state-render-facts.py --check",
        "",
        "#[rustfmt::skip]",
        "pub(super) const VANILLA_FACE_OCCLUSION_MASKS: &[[u64; 4]] = &[",
    ]
    lines.extend(
        "    [" + ", ".join(f"0x{word:016x}" for word in mask) + "],"
        for mask in palette
    )
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
    source = (
        args.facts_input.read_text()
        if args.facts_input is not None
        else runtime_facts(args)
    )
    facts = parse_facts(source, expected_state_count())
    palette, state_indices = build_palette(facts)
    probe_sha256 = hashlib.sha256(PROBE_SOURCE.read_text().encode()).hexdigest()
    protocol = render_protocol_rust(facts, state_indices, probe_sha256)
    renderer = render_renderer_rust(palette, probe_sha256)
    if args.check:
        protocol_ok = check_output(args.protocol_output, protocol)
        renderer_ok = check_output(args.renderer_output, renderer)
        return 0 if protocol_ok and renderer_ok else 1
    args.protocol_output.write_text(protocol)
    args.renderer_output.write_text(renderer)
    print(
        f"wrote {len(facts)} states and {len(palette)} face shapes to "
        f"{args.protocol_output} and {args.renderer_output}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
