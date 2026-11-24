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

from bbb_tools.vanilla.arguments import add_vanilla_runtime, resolve_runtime_inputs

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PROTOCOL_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/block_state_types/render_facts_26_1.rs"
)
DEFAULT_RENDER_BIN_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/data/block_state_render_facts_26_1.bin"
)
DEFAULT_OFFSET_BIN_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/data/block_state_model_offsets_26_1.bin"
)
DEFAULT_RENDERER_OUTPUT = (
    REPO_ROOT / "crates/bbb-client-render/src/terrain/face_occlusion_26_1.rs"
)
BLOCK_STATES = REPO_ROOT / "crates/bbb-facts/data/block_states_26_1.json"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockStateRenderFactsReport.java"

FaceMask = tuple[int, int, int, int]


@dataclass(frozen=True)
class StateRenderFacts:
    block_type_id: int
    behavior_bits: int
    light_emission: int
    emissive_rendering: bool
    face_masks: tuple[FaceMask, FaceMask, FaceMask, FaceMask, FaceMask, FaceMask]
    model_offset: tuple[float, float]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_runtime(parser)
    parser.add_argument("--facts-input", type=Path)
    parser.add_argument("--protocol-output", type=Path, default=DEFAULT_PROTOCOL_OUTPUT)
    parser.add_argument("--render-bin-output", type=Path)
    parser.add_argument("--offset-bin-output", type=Path)
    parser.add_argument("--renderer-output", type=Path, default=DEFAULT_RENDERER_OUTPUT)
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
        if len(fields) < 13 or fields[0] != str(expected_id):
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

        model_offset = (float(fields[5]), float(fields[6]))
        cursor = 7
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
                model_offset=model_offset,
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


def build_offset_runs(
    facts: tuple[StateRenderFacts, ...],
) -> tuple[tuple[int, tuple[float, float]], ...]:
    runs: list[tuple[int, tuple[float, float]]] = []
    for index, state in enumerate(facts):
        if index + 1 == len(facts) or facts[index + 1].model_offset != state.model_offset:
            runs.append((index, state.model_offset))
    return tuple(runs)


def pack_render_binary(
    facts: tuple[StateRenderFacts, ...],
    palette: tuple[FaceMask, ...],
    state_indices: tuple[tuple[int, int, int, int, int, int], ...],
) -> bytes:
    out = bytearray()
    for mask in palette:
        out.extend(struct.pack("<4Q", *mask))
    for state, face_indices in zip(facts, state_indices, strict=True):
        light_and_emissive = state.light_emission | (int(state.emissive_rendering) << 4)
        out.extend(
            struct.pack(
                "<HHB6B",
                state.block_type_id,
                state.behavior_bits,
                light_and_emissive,
                *face_indices,
            )
        )
    return bytes(out)


def pack_offset_binary(facts: tuple[StateRenderFacts, ...]) -> bytes:
    out = bytearray()
    for end, (horizontal, vertical) in build_offset_runs(facts):
        out.extend(struct.pack("<Hff", end, horizontal, vertical))
    return bytes(out)


def render_protocol_rust(
    facts: tuple[StateRenderFacts, ...],
    palette: tuple[FaceMask, ...],
    state_indices: tuple[tuple[int, int, int, int, int, int], ...],
    probe_sha256: str,
    render_bin_rel: str = "../../data/block_state_render_facts_26_1.bin",
    offset_bin_rel: str = "../../data/block_state_model_offsets_26_1.bin",
) -> str:
    _ = state_indices
    palette_len = len(palette)
    state_count = len(facts)
    states_offset = palette_len * 32
    offset_run_count = len(build_offset_runs(facts))
    lines = [
        "// @generated by scripts/generate-block-state-render-facts.py; do not edit manually.",
        "// Source: Vanilla 26.1 block identity, light-coordinate state facts, light-shape use, skipRendering class semantics, FluidRenderer overlay relation, and BlockState.getFaceOcclusionShape().",
        f"// Probe SHA-256: {probe_sha256}",
        "// Update: scripts/generate-block-state-render-facts.py",
        "// Check: scripts/generate-block-state-render-facts.py --check",
        "",
        f'const RENDER_BYTES: &[u8] = include_bytes!("{render_bin_rel}");',
        f"pub(super) const PALETTE_LEN: usize = {palette_len};",
        f"pub(super) const STATE_COUNT: usize = {state_count};",
        f"const STATES_OFFSET: usize = {states_offset};",
        "",
        '#[cfg(feature = "client-facts")]',
        f'const OFFSET_BYTES: &[u8] = include_bytes!("{offset_bin_rel}");',
        '#[cfg(feature = "client-facts")]',
        f"const MODEL_OFFSET_RUN_COUNT: usize = {offset_run_count};",
        "",
        "const fn u64_at(offset: usize) -> u64 {",
        "    u64::from_le_bytes([",
        "        RENDER_BYTES[offset],",
        "        RENDER_BYTES[offset + 1],",
        "        RENDER_BYTES[offset + 2],",
        "        RENDER_BYTES[offset + 3],",
        "        RENDER_BYTES[offset + 4],",
        "        RENDER_BYTES[offset + 5],",
        "        RENDER_BYTES[offset + 6],",
        "        RENDER_BYTES[offset + 7],",
        "    ])",
        "}",
        "",
        "const fn decode_face_occlusion_masks() -> [[u64; 4]; PALETTE_LEN] {",
        "    let mut masks = [[0u64; 4]; PALETTE_LEN];",
        "    let mut index = 0;",
        "    while index < PALETTE_LEN {",
        "        let base = index * 32;",
        "        masks[index] = [",
        "            u64_at(base),",
        "            u64_at(base + 8),",
        "            u64_at(base + 16),",
        "            u64_at(base + 24),",
        "        ];",
        "        index += 1;",
        "    }",
        "    masks",
        "}",
        "",
        "static DECODED_FACE_OCCLUSION_MASKS: [[u64; 4]; PALETTE_LEN] = decode_face_occlusion_masks();",
        "pub(super) const VANILLA_FACE_OCCLUSION_MASKS: &[[u64; 4]] = &DECODED_FACE_OCCLUSION_MASKS;",
        "",
        "#[inline]",
        "pub(super) fn render_block_id(state_id: usize) -> Option<u16> {",
        "    if state_id >= STATE_COUNT {",
        "        return None;",
        "    }",
        "    let offset = STATES_OFFSET + state_id * 11;",
        "    Some(u16::from_le_bytes([",
        "        RENDER_BYTES[offset],",
        "        RENDER_BYTES[offset + 1],",
        "    ]))",
        "}",
        "",
        "#[inline]",
        "pub(super) fn render_state_record(state_id: usize) -> Option<(u16, u16, u8, bool, [u8; 6])> {",
        "    if state_id >= STATE_COUNT {",
        "        return None;",
        "    }",
        "    let offset = STATES_OFFSET + state_id * 11;",
        "    let block_type_id = u16::from_le_bytes([RENDER_BYTES[offset], RENDER_BYTES[offset + 1]]);",
        "    let behavior_bits = u16::from_le_bytes([RENDER_BYTES[offset + 2], RENDER_BYTES[offset + 3]]);",
        "    let light_and_emissive = RENDER_BYTES[offset + 4];",
        "    let light_emission = light_and_emissive & 0x0f;",
        "    let emissive_rendering = light_and_emissive & 0x10 != 0;",
        "    let faces = [",
        "        RENDER_BYTES[offset + 5],",
        "        RENDER_BYTES[offset + 6],",
        "        RENDER_BYTES[offset + 7],",
        "        RENDER_BYTES[offset + 8],",
        "        RENDER_BYTES[offset + 9],",
        "        RENDER_BYTES[offset + 10],",
        "    ];",
        "    Some((",
        "        block_type_id,",
        "        behavior_bits,",
        "        light_emission,",
        "        emissive_rendering,",
        "        faces,",
        "    ))",
        "}",
        "",
        '#[cfg(feature = "client-facts")]',
        "#[inline]",
        "fn model_offset_run_end(index: usize) -> u16 {",
        "    let offset = index * 10;",
        "    u16::from_le_bytes([OFFSET_BYTES[offset], OFFSET_BYTES[offset + 1]])",
        "}",
        "",
        '#[cfg(feature = "client-facts")]',
        "pub(super) fn model_offset_limits(state_id: u16) -> Option<[f32; 2]> {",
        "    let mut lo = 0usize;",
        "    let mut hi = MODEL_OFFSET_RUN_COUNT;",
        "    while lo < hi {",
        "        let mid = lo + (hi - lo) / 2;",
        "        if model_offset_run_end(mid) < state_id {",
        "            lo = mid + 1;",
        "        } else {",
        "            hi = mid;",
        "        }",
        "    }",
        "    if lo >= MODEL_OFFSET_RUN_COUNT {",
        "        return None;",
        "    }",
        "    let offset = lo * 10;",
        "    let horiz = f32::from_le_bytes([",
        "        OFFSET_BYTES[offset + 2],",
        "        OFFSET_BYTES[offset + 3],",
        "        OFFSET_BYTES[offset + 4],",
        "        OFFSET_BYTES[offset + 5],",
        "    ]);",
        "    let vert = f32::from_le_bytes([",
        "        OFFSET_BYTES[offset + 6],",
        "        OFFSET_BYTES[offset + 7],",
        "        OFFSET_BYTES[offset + 8],",
        "        OFFSET_BYTES[offset + 9],",
        "    ]);",
        "    Some([horiz, vert])",
        "}",
        "",
    ]
    return "\n".join(lines)


def render_renderer_rust(probe_sha256: str) -> str:
    lines = [
        "// @generated by scripts/generate-block-state-render-facts.py; do not edit manually.",
        "// The exact face occupancy palette is owned by bbb-facts.",
        f"// Probe SHA-256: {probe_sha256}",
        "// Update: scripts/generate-block-state-render-facts.py",
        "// Check: scripts/generate-block-state-render-facts.py --check",
        "",
        "pub(super) fn vanilla_face_occlusion_masks() -> &'static [[u64; 4]] {",
        "    bbb_protocol::block_state_types::vanilla_face_occlusion_masks()",
        "}",
        "",
    ]
    return "\n".join(lines)


def _resolve_render_bins(
    protocol_output: Path,
    render_bin_output: Path | None,
    offset_bin_output: Path | None,
) -> tuple[Path, str, Path, str]:
    if render_bin_output is None:
        render_bin_output = (
            DEFAULT_RENDER_BIN_OUTPUT
            if protocol_output == DEFAULT_PROTOCOL_OUTPUT
            else protocol_output.with_name(protocol_output.stem + "_render.bin")
        )
    if offset_bin_output is None:
        offset_bin_output = (
            DEFAULT_OFFSET_BIN_OUTPUT
            if protocol_output == DEFAULT_PROTOCOL_OUTPUT
            else protocol_output.with_name(protocol_output.stem + "_offsets.bin")
        )
    render_rel = os.path.relpath(render_bin_output, protocol_output.parent).replace(
        "\\", "/"
    )
    offset_rel = os.path.relpath(offset_bin_output, protocol_output.parent).replace(
        "\\", "/"
    )
    return render_bin_output, render_rel, offset_bin_output, offset_rel


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
    source = (
        args.facts_input.read_text()
        if args.facts_input is not None
        else runtime_facts(args)
    )
    facts = parse_facts(source, expected_state_count())
    palette, state_indices = build_palette(facts)
    probe_sha256 = hashlib.sha256(PROBE_SOURCE.read_text().encode()).hexdigest()
    render_bin, render_rel, offset_bin, offset_rel = _resolve_render_bins(
        args.protocol_output, args.render_bin_output, args.offset_bin_output
    )
    protocol = render_protocol_rust(
        facts, palette, state_indices, probe_sha256, render_rel, offset_rel
    )
    render_blob = pack_render_binary(facts, palette, state_indices)
    offset_blob = pack_offset_binary(facts)
    renderer = render_renderer_rust(probe_sha256)
    if args.check:
        protocol_ok = check_output(args.protocol_output, protocol)
        render_bin_ok = check_bytes_output(render_bin, render_blob)
        offset_bin_ok = check_bytes_output(offset_bin, offset_blob)
        renderer_ok = check_output(args.renderer_output, renderer)
        return 0 if protocol_ok and render_bin_ok and offset_bin_ok and renderer_ok else 1
    args.protocol_output.parent.mkdir(parents=True, exist_ok=True)
    render_bin.parent.mkdir(parents=True, exist_ok=True)
    offset_bin.parent.mkdir(parents=True, exist_ok=True)
    args.protocol_output.write_text(protocol)
    render_bin.write_bytes(render_blob)
    offset_bin.write_bytes(offset_blob)
    args.renderer_output.write_text(renderer)
    print(
        f"wrote {len(facts)} states and {len(palette)} face shapes to "
        f"{args.protocol_output} and {args.renderer_output}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
