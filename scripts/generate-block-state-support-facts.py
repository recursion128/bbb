#!/usr/bin/env python3
"""Generate Vanilla 26.1 block-state support facts."""

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
    REPO_ROOT / "crates/bbb-facts/src/block_state_types/support_facts_26_1.rs"
)
DEFAULT_BIN_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/data/block_state_support_facts_26_1.bin"
)
DEFAULT_MAP_COLOR_BIN_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/data/block_state_map_colors_26_1.bin"
)
BLOCK_STATES = REPO_ROOT / "crates/bbb-facts/data/block_states_26_1.json"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockStateSupportReport.java"
PISTON_REACTION_IDS = {
    "NORMAL": 0,
    "DESTROY": 1,
    "BLOCK": 2,
    "IGNORE": 3,
    "PUSH_ONLY": 4,
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_runtime(parser)
    parser.add_argument("--facts-input", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--bin-output", type=Path)
    parser.add_argument("--map-color-bin-output", type=Path)
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
    with tempfile.TemporaryDirectory(prefix="bbb-block-state-support-") as directory:
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
                "BlockStateSupportReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla block-state support probe failed:\n" + result.stderr.strip()
            )
    return result.stdout


def parse_facts(
    source: str, expected_count: int
) -> tuple[
    tuple[int, ...],
    tuple[bool, ...],
    tuple[bool, ...],
    tuple[bool, ...],
    tuple[bool, ...],
    tuple[bool, ...],
    tuple[int, ...],
    tuple[int, ...],
    tuple[bool, ...],
    tuple[bool, ...],
    tuple[int | None, ...],
    tuple[int | None, ...],
    tuple[int, ...],
    tuple[int, ...],
]:
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

    parsed_facts = []
    can_occlude = []
    solid_render = []
    redstone_conductor = []
    suffocating = []
    view_blocking = []
    light_dampening = []
    shade_brightness = []
    liquid = []
    falling_dust_accepts = []
    falling_block_dust_colors = []
    static_map_colors = []
    piston_facts = []
    map_color_ids = []
    for expected_id, line in enumerate(lines[1:]):
        fields = line.split("\t")
        if len(fields) != 16 or fields[0] != str(expected_id):
            raise ValueError(f"invalid runtime fact at id {expected_id}: {line!r}")
        try:
            facts = int(fields[1])
        except ValueError as error:
            raise ValueError(
                f"invalid runtime fact at id {expected_id}: {line!r}"
            ) from error
        if not 0 <= facts <= 0xFFFF_FFFF_FFFF_FFFF:
            raise ValueError(
                f"runtime fact outside u64 at id {expected_id}: {facts}"
            )
        if fields[2] not in ("0", "1"):
            raise ValueError(
                f"invalid can-occlude fact at id {expected_id}: {fields[2]!r}"
            )
        if fields[3] not in ("0", "1"):
            raise ValueError(
                f"invalid solid-render fact at id {expected_id}: {fields[3]!r}"
            )
        if fields[4] not in ("0", "1"):
            raise ValueError(
                f"invalid redstone-conductor fact at id {expected_id}: "
                f"{fields[4]!r}"
            )
        if fields[5] not in ("0", "1"):
            raise ValueError(
                f"invalid suffocating fact at id {expected_id}: {fields[5]!r}"
            )
        if fields[6] not in ("0", "1"):
            raise ValueError(
                f"invalid view-blocking fact at id {expected_id}: {fields[6]!r}"
            )
        try:
            dampening = int(fields[7])
            brightness = int(fields[8])
        except ValueError as error:
            raise ValueError(
                f"invalid lighting fact at id {expected_id}: {line!r}"
            ) from error
        if not 0 <= dampening <= 15:
            raise ValueError(
                f"light-dampening fact outside u4 at id {expected_id}: {dampening}"
            )
        if not 0 <= brightness <= 255:
            raise ValueError(
                f"shade-brightness fact outside u8 at id {expected_id}: {brightness}"
            )
        if fields[9] not in ("0", "1"):
            raise ValueError(
                f"invalid liquid fact at id {expected_id}: {fields[9]!r}"
            )
        if fields[10] not in ("0", "1"):
            raise ValueError(
                f"invalid falling-dust acceptance at id {expected_id}: "
                f"{fields[10]!r}"
            )
        try:
            falling_color = None if fields[11] == "-" else int(fields[11])
            map_color = None if fields[12] == "-" else int(fields[12])
        except ValueError as error:
            raise ValueError(
                f"invalid falling-dust color at id {expected_id}: {line!r}"
            ) from error
        if falling_color is not None and not 0 <= falling_color <= 0xFF_FFFF:
            raise ValueError(
                f"falling-block dust color outside RGB24 at id {expected_id}: "
                f"{falling_color}"
            )
        if map_color is not None and not 0 <= map_color <= 0xFF_FFFF:
            raise ValueError(
                f"static map color outside RGB24 at id {expected_id}: {map_color}"
            )
        reaction = PISTON_REACTION_IDS.get(fields[13])
        if reaction is None:
            raise ValueError(
                f"invalid piston push reaction at id {expected_id}: {fields[13]!r}"
            )
        if fields[14] not in ("0", "1"):
            raise ValueError(
                f"invalid piston unbreakable fact at id {expected_id}: {fields[14]!r}"
            )
        try:
            map_color_id = int(fields[15])
        except ValueError as error:
            raise ValueError(
                f"invalid map material id at id {expected_id}: {fields[15]!r}"
            ) from error
        if not 0 <= map_color_id <= 63:
            raise ValueError(
                f"map material id outside u6 at id {expected_id}: {map_color_id}"
            )
        map_color_ids.append(map_color_id)
        can_occlude.append(fields[2] == "1")
        solid_render.append(fields[3] == "1")
        redstone_conductor.append(fields[4] == "1")
        suffocating.append(fields[5] == "1")
        view_blocking.append(fields[6] == "1")
        light_dampening.append(dampening)
        shade_brightness.append(brightness)
        liquid.append(fields[9] == "1")
        falling_dust_accepts.append(fields[10] == "1")
        falling_block_dust_colors.append(falling_color)
        static_map_colors.append(map_color)
        piston_facts.append(reaction | ((fields[14] == "1") << 3))
        parsed_facts.append(facts)
    return (
        tuple(parsed_facts),
        tuple(can_occlude),
        tuple(solid_render),
        tuple(redstone_conductor),
        tuple(suffocating),
        tuple(view_blocking),
        tuple(light_dampening),
        tuple(shade_brightness),
        tuple(liquid),
        tuple(falling_dust_accepts),
        tuple(falling_block_dust_colors),
        tuple(static_map_colors),
        tuple(piston_facts),
        tuple(map_color_ids),
    )


def _prepare_tables(
    facts: tuple[int, ...],
    can_occlude: tuple[bool, ...],
    solid_render: tuple[bool, ...],
    redstone_conductor: tuple[bool, ...],
    suffocating: tuple[bool, ...],
    view_blocking: tuple[bool, ...],
    light_dampening: tuple[int, ...],
    shade_brightness: tuple[int, ...],
    liquid: tuple[bool, ...],
    falling_dust_accepts: tuple[bool, ...],
    falling_block_dust_colors: tuple[int | None, ...],
    static_map_colors: tuple[int | None, ...],
    piston_facts: tuple[int, ...],
    map_color_ids: tuple[int, ...],
) -> tuple[
    tuple[int, ...],
    tuple[int, ...],
    tuple[tuple[int, int], ...],
    tuple[int, ...],
    tuple[int, ...],
]:
    if (
        len(facts) != len(solid_render)
        or len(facts) != len(can_occlude)
        or len(facts) != len(redstone_conductor)
        or len(facts) != len(suffocating)
        or len(facts) != len(view_blocking)
        or len(facts) != len(light_dampening)
        or len(facts) != len(shade_brightness)
        or len(facts) != len(liquid)
        or len(facts) != len(falling_dust_accepts)
        or len(facts) != len(falling_block_dust_colors)
        or len(facts) != len(static_map_colors)
        or len(facts) != len(piston_facts)
        or len(facts) != len(map_color_ids)
    ):
        raise ValueError(
            "support, can-occlude, solid-render, redstone-conductor, suffocating, "
            "view-blocking, light-dampening, shade-brightness, liquid, piston, "
            "and map material facts "
            "have different lengths"
        )
    if any(value & ~0x0F for value in piston_facts):
        raise ValueError("piston facts do not fit in four bits")
    if any(value & ~0x0F for value in light_dampening):
        raise ValueError("light dampening does not fit in four bits")
    palette = tuple(dict.fromkeys(facts))
    if len(palette) > 256:
        raise ValueError(
            f"support fact palette has {len(palette)} entries and does not fit in u8"
        )
    palette_indices = {value: index for index, value in enumerate(palette)}
    state_indices = tuple(palette_indices[value] for value in facts)
    falling_colors_by_state = tuple(
        (state_id, color)
        for state_id, color in enumerate(falling_block_dust_colors)
        if color is not None
    )
    map_color_palette = tuple(
        dict.fromkeys(color for color in static_map_colors if color is not None)
    )
    if len(map_color_palette) > 255:
        raise ValueError("optional static map color palette does not fit in u8 indices")
    map_color_indices = {
        value: index + 1 for index, value in enumerate(map_color_palette)
    }
    map_state_indices = tuple(
        0 if value is None else map_color_indices[value] for value in static_map_colors
    )
    return (
        palette,
        state_indices,
        falling_colors_by_state,
        map_color_palette,
        map_state_indices,
    )


def pack_support_binary(
    facts: tuple[int, ...],
    can_occlude: tuple[bool, ...],
    solid_render: tuple[bool, ...],
    redstone_conductor: tuple[bool, ...],
    suffocating: tuple[bool, ...],
    view_blocking: tuple[bool, ...],
    light_dampening: tuple[int, ...],
    shade_brightness: tuple[int, ...],
    liquid: tuple[bool, ...],
    falling_dust_accepts: tuple[bool, ...],
    falling_block_dust_colors: tuple[int | None, ...],
    static_map_colors: tuple[int | None, ...],
    piston_facts: tuple[int, ...],
    map_color_ids: tuple[int, ...],
) -> tuple[bytes, bytes]:
    (
        palette,
        state_indices,
        falling_colors_by_state,
        map_color_palette,
        map_state_indices,
    ) = _prepare_tables(
        facts,
        can_occlude,
        solid_render,
        redstone_conductor,
        suffocating,
        view_blocking,
        light_dampening,
        shade_brightness,
        liquid,
        falling_dust_accepts,
        falling_block_dust_colors,
        static_map_colors,
        piston_facts,
        map_color_ids,
    )
    out = bytearray()
    for value in palette:
        out.extend(struct.pack("<Q", value))
    for color in map_color_palette:
        out.extend(struct.pack("<I", color))
    for state_id, color in falling_colors_by_state:
        out.extend(struct.pack("<HI", state_id, color))
    for state_id, support_index in enumerate(state_indices):
        flags = (
            int(can_occlude[state_id])
            | (int(solid_render[state_id]) << 1)
            | (int(redstone_conductor[state_id]) << 2)
            | (int(suffocating[state_id]) << 3)
            | (int(view_blocking[state_id]) << 4)
            | (int(liquid[state_id]) << 5)
            | (int(falling_dust_accepts[state_id]) << 6)
        )
        dampening_piston = light_dampening[state_id] | (piston_facts[state_id] << 4)
        out.extend(
            struct.pack(
                "<5B",
                support_index,
                flags,
                dampening_piston,
                shade_brightness[state_id],
                map_state_indices[state_id],
            )
        )
    return bytes(out), bytes(map_color_ids)


def render_rust(
    facts: tuple[int, ...],
    can_occlude: tuple[bool, ...],
    solid_render: tuple[bool, ...],
    redstone_conductor: tuple[bool, ...],
    suffocating: tuple[bool, ...],
    view_blocking: tuple[bool, ...],
    light_dampening: tuple[int, ...],
    shade_brightness: tuple[int, ...],
    liquid: tuple[bool, ...],
    falling_dust_accepts: tuple[bool, ...],
    falling_block_dust_colors: tuple[int | None, ...],
    static_map_colors: tuple[int | None, ...],
    piston_facts: tuple[int, ...],
    map_color_ids: tuple[int, ...],
    bin_rel_path: str = "../../data/block_state_support_facts_26_1.bin",
    map_color_bin_rel_path: str = "../../data/block_state_map_colors_26_1.bin",
) -> str:
    (
        palette,
        state_indices,
        falling_colors_by_state,
        map_color_palette,
        _map_state_indices,
    ) = _prepare_tables(
        facts,
        can_occlude,
        solid_render,
        redstone_conductor,
        suffocating,
        view_blocking,
        light_dampening,
        shade_brightness,
        liquid,
        falling_dust_accepts,
        falling_block_dust_colors,
        static_map_colors,
        piston_facts,
        map_color_ids,
    )
    support_palette_len = len(palette)
    static_map_palette_len = len(map_color_palette)
    falling_dust_count = len(falling_colors_by_state)
    state_count = len(state_indices)
    static_map_palette_offset = support_palette_len * 8
    falling_dust_offset = static_map_palette_offset + static_map_palette_len * 4
    states_offset = falling_dust_offset + falling_dust_count * 6
    lines = [
        "// @generated by scripts/generate-block-state-support-facts.py; do not edit manually.",
        "// Source: Vanilla 26.1 BlockState support, collision, occlusion-face, replaceable, can-occlude, solid-render, redstone-conductor, suffocating, view-blocking, light-dampening, shade-brightness, liquid, falling-dust static, piston, and map material behavior, evaluated by the checked-in runtime probe.",
        "// Dynamic-shape states are marked; their EmptyBlockGetter-derived shape bits are not live-world answers.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-block-state-support-facts.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-block-state-support-facts.py --check",
        "",
        f'const BYTES: &[u8] = include_bytes!("{bin_rel_path}");',
        f"pub(super) const SUPPORT_PALETTE_LEN: usize = {support_palette_len};",
        f"pub(super) const STATIC_MAP_COLOR_PALETTE_LEN: usize = {static_map_palette_len};",
        f"pub(super) const FALLING_DUST_COLOR_COUNT: usize = {falling_dust_count};",
        f"pub(super) const STATE_COUNT: usize = {state_count};",
        f"const STATIC_MAP_PALETTE_OFFSET: usize = {static_map_palette_offset};",
        f"const FALLING_DUST_OFFSET: usize = {falling_dust_offset};",
        f"const STATES_OFFSET: usize = {states_offset};",
        "",
        '#[cfg(feature = "server-facts")]',
        f'const MAP_COLOR_BYTES: &[u8] = include_bytes!("{map_color_bin_rel_path}");',
        "",
        "#[inline]",
        "pub(super) fn support_palette_entry(index: usize) -> Option<u64> {",
        "    if index >= SUPPORT_PALETTE_LEN {",
        "        return None;",
        "    }",
        "    let offset = index * 8;",
        "    Some(u64::from_le_bytes([",
        "        BYTES[offset],",
        "        BYTES[offset + 1],",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "        BYTES[offset + 4],",
        "        BYTES[offset + 5],",
        "        BYTES[offset + 6],",
        "        BYTES[offset + 7],",
        "    ]))",
        "}",
        "",
        "#[inline]",
        "pub(super) fn state_raw_record(state_id: usize) -> Option<[u8; 5]> {",
        "    if state_id >= STATE_COUNT {",
        "        return None;",
        "    }",
        "    let offset = STATES_OFFSET + state_id * 5;",
        "    Some([",
        "        BYTES[offset],",
        "        BYTES[offset + 1],",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "        BYTES[offset + 4],",
        "    ])",
        "}",
        "",
        "#[inline]",
        "pub(super) fn static_map_color_rgb(one_based_index: u8) -> Option<u32> {",
        "    let index = usize::from(one_based_index).checked_sub(1)?;",
        "    if index >= STATIC_MAP_COLOR_PALETTE_LEN {",
        "        return None;",
        "    }",
        "    let offset = STATIC_MAP_PALETTE_OFFSET + index * 4;",
        "    Some(u32::from_le_bytes([",
        "        BYTES[offset],",
        "        BYTES[offset + 1],",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "    ]))",
        "}",
        "",
        "#[inline]",
        "pub(super) fn falling_dust_entry(index: usize) -> Option<(u16, u32)> {",
        "    if index >= FALLING_DUST_COLOR_COUNT {",
        "        return None;",
        "    }",
        "    let offset = FALLING_DUST_OFFSET + index * 6;",
        "    let state_id = u16::from_le_bytes([BYTES[offset], BYTES[offset + 1]]);",
        "    let color = u32::from_le_bytes([",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "        BYTES[offset + 4],",
        "        BYTES[offset + 5],",
        "    ]);",
        "    Some((state_id, color))",
        "}",
        "",
        "pub(super) fn falling_block_dust_color_rgb(state_id: u16) -> Option<u32> {",
        "    let mut lo = 0usize;",
        "    let mut hi = FALLING_DUST_COLOR_COUNT;",
        "    while lo < hi {",
        "        let mid = lo + (hi - lo) / 2;",
        "        let (entry_state, color) = falling_dust_entry(mid)?;",
        "        if entry_state < state_id {",
        "            lo = mid + 1;",
        "        } else if entry_state > state_id {",
        "            hi = mid;",
        "        } else {",
        "            return Some(color);",
        "        }",
        "    }",
        "    None",
        "}",
        "",
        '#[cfg(feature = "server-facts")]',
        "#[inline]",
        "pub(super) fn map_color_id(state_id: usize) -> Option<u8> {",
        "    MAP_COLOR_BYTES.get(state_id).copied()",
        "}",
        "",
    ]
    return "\n".join(lines)


def _resolve_support_bins(
    output: Path,
    bin_output: Path | None,
    map_color_bin_output: Path | None,
) -> tuple[Path, str, Path, str]:
    if bin_output is None:
        bin_output = (
            DEFAULT_BIN_OUTPUT
            if output == DEFAULT_OUTPUT
            else output.with_name(output.stem + "_support.bin")
        )
    if map_color_bin_output is None:
        map_color_bin_output = (
            DEFAULT_MAP_COLOR_BIN_OUTPUT
            if output == DEFAULT_OUTPUT
            else output.with_name(output.stem + "_map_colors.bin")
        )
    bin_rel = os.path.relpath(bin_output, output.parent).replace("\\", "/")
    map_rel = os.path.relpath(map_color_bin_output, output.parent).replace("\\", "/")
    return bin_output, bin_rel, map_color_bin_output, map_rel


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
    parsed = parse_facts(facts, count)
    bin_output, bin_rel, map_bin_output, map_rel = _resolve_support_bins(
        args.output, args.bin_output, args.map_color_bin_output
    )
    generated = render_rust(*parsed, bin_rel, map_rel)
    support_blob, map_blob = pack_support_binary(*parsed)
    if args.check:
        rust_ok = check_output(args.output, generated)
        bin_ok = check_bytes_output(bin_output, support_blob)
        map_ok = check_bytes_output(map_bin_output, map_blob)
        return 0 if rust_ok and bin_ok and map_ok else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    bin_output.parent.mkdir(parents=True, exist_ok=True)
    map_bin_output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    bin_output.write_bytes(support_blob)
    map_bin_output.write_bytes(map_blob)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
