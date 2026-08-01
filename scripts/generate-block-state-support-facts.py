#!/usr/bin/env python3
"""Generate Vanilla 26.1 block-state support facts."""

from __future__ import annotations

import argparse
import difflib
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
    REPO_ROOT / "crates/bbb-protocol/src/block_state_types/support_facts_26_1.rs"
)
BLOCK_STATES = REPO_ROOT / "crates/bbb-protocol/data/block_states_26_1.json"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockStateSupportReport.java"


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
    tuple[bool, ...],
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
    for expected_id, line in enumerate(lines[1:]):
        fields = line.split("\t")
        if len(fields) != 10 or fields[0] != str(expected_id):
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
        can_occlude.append(fields[2] == "1")
        solid_render.append(fields[3] == "1")
        redstone_conductor.append(fields[4] == "1")
        suffocating.append(fields[5] == "1")
        view_blocking.append(fields[6] == "1")
        light_dampening.append(dampening)
        shade_brightness.append(brightness)
        liquid.append(fields[9] == "1")
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
    )


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
) -> str:
    if (
        len(facts) != len(solid_render)
        or len(facts) != len(can_occlude)
        or len(facts) != len(redstone_conductor)
        or len(facts) != len(suffocating)
        or len(facts) != len(view_blocking)
        or len(facts) != len(light_dampening)
        or len(facts) != len(shade_brightness)
        or len(facts) != len(liquid)
    ):
        raise ValueError(
            "support, can-occlude, solid-render, redstone-conductor, suffocating, "
            "view-blocking, light-dampening, shade-brightness, and liquid facts "
            "have different lengths"
        )
    palette = tuple(dict.fromkeys(facts))
    if len(palette) > 256:
        raise ValueError(
            f"support fact palette has {len(palette)} entries and does not fit in u8"
        )
    palette_indices = {value: index for index, value in enumerate(palette)}
    state_indices = tuple(palette_indices[value] for value in facts)
    lines = [
        "// @generated by scripts/generate-block-state-support-facts.py; do not edit manually.",
        "// Source: Vanilla 26.1 BlockState support, collision, occlusion-face, replaceable, can-occlude, solid-render, redstone-conductor, suffocating, view-blocking, light-dampening, shade-brightness, and liquid behavior, evaluated by the checked-in runtime probe.",
        "// Dynamic-shape states are marked; their EmptyBlockGetter-derived shape bits are not live-world answers.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-block-state-support-facts.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-block-state-support-facts.py --check",
        "",
        "#[rustfmt::skip]",
        "pub(super) const VANILLA_BLOCK_STATE_SUPPORT_FACT_PALETTE: &[u64] = &[",
    ]
    lines.extend(f"    0x{value:016x}," for value in palette)
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_SUPPORT_FACT_INDICES: &[u8] = &[",
        )
    )
    for start in range(0, len(state_indices), 16):
        row = state_indices[start : start + 16]
        lines.append("    " + ", ".join(str(index) for index in row) + ",")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_CAN_OCCLUDE_BITS: &[u64] = &[",
        )
    )
    for start in range(0, len(can_occlude), 64):
        word = sum(
            1 << bit
            for bit, value in enumerate(can_occlude[start : start + 64])
            if value
        )
        lines.append(f"    0x{word:016x},")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_SOLID_RENDER_BITS: &[u64] = &[",
        )
    )
    for start in range(0, len(solid_render), 64):
        word = sum(
            1 << bit
            for bit, value in enumerate(solid_render[start : start + 64])
            if value
        )
        lines.append(f"    0x{word:016x},")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_REDSTONE_CONDUCTOR_BITS: &[u64] = &[",
        )
    )
    for start in range(0, len(redstone_conductor), 64):
        word = sum(
            1 << bit
            for bit, value in enumerate(redstone_conductor[start : start + 64])
            if value
        )
        lines.append(f"    0x{word:016x},")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_SUFFOCATING_BITS: &[u64] = &[",
        )
    )
    for start in range(0, len(suffocating), 64):
        word = sum(
            1 << bit
            for bit, value in enumerate(suffocating[start : start + 64])
            if value
        )
        lines.append(f"    0x{word:016x},")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_VIEW_BLOCKING_BITS: &[u64] = &[",
        )
    )
    for start in range(0, len(view_blocking), 64):
        word = sum(
            1 << bit
            for bit, value in enumerate(view_blocking[start : start + 64])
            if value
        )
        lines.append(f"    0x{word:016x},")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_LIGHT_DAMPENING: &[u8] = &[",
        )
    )
    for start in range(0, len(light_dampening), 32):
        row = light_dampening[start : start + 32]
        lines.append("    " + ", ".join(str(value) for value in row) + ",")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_SHADE_BRIGHTNESS: &[u8] = &[",
        )
    )
    for start in range(0, len(shade_brightness), 32):
        row = shade_brightness[start : start + 32]
        lines.append("    " + ", ".join(str(value) for value in row) + ",")
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_STATE_LIQUID_BITS: &[u64] = &[",
        )
    )
    for start in range(0, len(liquid), 64):
        word = sum(
            1 << bit
            for bit, value in enumerate(liquid[start : start + 64])
            if value
        )
        lines.append(f"    0x{word:016x},")
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
    (
        support_facts,
        can_occlude,
        solid_render,
        redstone_conductor,
        suffocating,
        view_blocking,
        light_dampening,
        shade_brightness,
        liquid,
    ) = parse_facts(facts, count)
    generated = render_rust(
        support_facts,
        can_occlude,
        solid_render,
        redstone_conductor,
        suffocating,
        view_blocking,
        light_dampening,
        shade_brightness,
        liquid,
    )
    if args.check:
        return 0 if check_output(args.output, generated) else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
