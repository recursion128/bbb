#!/usr/bin/env python3
"""Generate Vanilla 26.1 BlockItem default placement facts."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DOWNLOAD_ROOT = Path.home() / "Work/mc-code/downloads/26.1"
DEFAULT_JDK_ROOT = Path.home() / ".local/jdks/jdk-25.0.3+9"
DEFAULT_OUTPUT = (
    REPO_ROOT
    / "crates/bbb-protocol/src/block_state_types/placement_defaults_26_1.rs"
)
BLOCK_STATES = REPO_ROOT / "crates/bbb-protocol/data/block_states_26_1.json"
BLOCK_CATALOG = (
    REPO_ROOT / "crates/bbb-pack/data/block_registry_declarations_26_1.json"
)
PROBE_SOURCE = (
    REPO_ROOT / "scripts/vanilla-runtime/BlockPlacementDefaultsReport.java"
)
EXPECTED_ITEM_COUNT = 1506
EXPECTED_ITEM_BLOCK_STATE_COMPONENT_COUNT = 12
RESOURCE_ID = re.compile(r"[a-z0-9_.-]+:[a-z0-9_./-]+")
PROPERTY_TOKEN = re.compile(r"[a-z0-9_]+")


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


def expected_catalogs() -> tuple[list[dict], list[str]]:
    state_report = json.loads(BLOCK_STATES.read_text())
    if state_report.get("version") != "26.1":
        raise ValueError("embedded block-state report is not for Vanilla 26.1")
    states = state_report.get("states")
    if not isinstance(states, list):
        raise ValueError("embedded block-state report lacks states")
    for expected_id, state in enumerate(states):
        if state.get("id") != expected_id:
            raise ValueError("embedded block-state report is not contiguous")

    block_report = json.loads(BLOCK_CATALOG.read_text())
    if block_report.get("vanilla_version") != "26.1":
        raise ValueError("generated block catalog is not for Vanilla 26.1")
    declarations = block_report.get("declarations")
    if not isinstance(declarations, list):
        raise ValueError("generated block catalog lacks declarations")
    resources = []
    for expected_id, declaration in enumerate(declarations):
        if declaration.get("declaration_order") != expected_id:
            raise ValueError("generated block catalog is not contiguous")
        resources.append(declaration["resource_id"])
    return states, resources


def runtime_facts(args: argparse.Namespace) -> str:
    jars = [args.client_jar, *sorted(args.libraries_root.rglob("*.jar"))]
    missing = [path for path in (args.javac, args.java, *jars[:1]) if not path.is_file()]
    if missing:
        raise FileNotFoundError(
            "missing Vanilla runtime probe input: "
            + ", ".join(str(path) for path in missing)
        )
    classpath = os.pathsep.join(str(path) for path in jars)
    with tempfile.TemporaryDirectory(prefix="bbb-block-placement-defaults-") as directory:
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
                "BlockPlacementDefaultsReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla block placement defaults probe failed:\n"
                + result.stderr.strip()
                + ("\nstdout:\n" + result.stdout.strip() if result.stdout else "")
            )
    return result.stdout


def parse_facts(
    source: str,
    states: list[dict],
    block_resources: list[str],
    expected_item_count: int = EXPECTED_ITEM_COUNT,
) -> tuple[
    tuple[int, ...],
    tuple[str, ...],
    tuple[tuple[int, tuple[tuple[str, str], ...]], ...],
]:
    lines = source.splitlines()
    expected_header = (
        f"version=26.1 blocks={len(block_resources)} "
        f"items={expected_item_count} states={len(states)}"
    )
    if not lines or lines[0] != expected_header:
        raise ValueError(
            f"expected runtime report header {expected_header!r}, got "
            f"{lines[0] if lines else '<empty>'!r}"
        )
    expected_lines = 1 + len(block_resources) + expected_item_count
    if len(lines) != expected_lines:
        raise ValueError(
            f"expected {expected_lines - 1} runtime facts, got {len(lines) - 1}"
        )

    defaults = []
    block_instruments = []
    for block_id, expected_resource in enumerate(block_resources):
        line = lines[1 + block_id]
        fields = line.split("\t")
        if (
            len(fields) != 5
            or fields[:3] != ["block", str(block_id), expected_resource]
        ):
            raise ValueError(f"invalid runtime block fact at id {block_id}: {line!r}")
        try:
            state_id = int(fields[3])
        except ValueError as error:
            raise ValueError(
                f"invalid default state id for block {block_id}: {line!r}"
            ) from error
        if not 0 <= state_id < len(states):
            raise ValueError(f"default state outside registry for block {block_id}")
        if states[state_id].get("name") != expected_resource:
            raise ValueError(
                f"default state {state_id} does not belong to {expected_resource}"
            )
        if PROPERTY_TOKEN.fullmatch(fields[4]) is None:
            raise ValueError(
                f"invalid note-block instrument for block {block_id}: {fields[4]!r}"
            )
        defaults.append(state_id)
        block_instruments.append(fields[4])

    item_defaults = []
    offset = 1 + len(block_resources)
    for item_id, line in enumerate(lines[offset:]):
        fields = line.split("\t")
        if (
            len(fields) != 4
            or fields[0] != "item"
            or fields[1] != str(item_id)
            or RESOURCE_ID.fullmatch(fields[2]) is None
        ):
            raise ValueError(f"invalid runtime item fact at id {item_id}: {line!r}")
        if fields[3] == "-":
            continue
        properties = []
        for entry in fields[3].split(","):
            name, separator, value = entry.partition("=")
            if (
                not separator
                or PROPERTY_TOKEN.fullmatch(name) is None
                or PROPERTY_TOKEN.fullmatch(value) is None
            ):
                raise ValueError(
                    f"invalid BLOCK_STATE property for item {item_id}: {entry!r}"
                )
            properties.append((name, value))
        if properties != sorted(set(properties)):
            raise ValueError(
                f"BLOCK_STATE properties are not sorted and unique for item {item_id}"
            )
        item_defaults.append((item_id, tuple(properties)))
    return tuple(defaults), tuple(block_instruments), tuple(item_defaults)


def render_rust(
    defaults: tuple[int, ...],
    block_instruments: tuple[str, ...],
    item_defaults: tuple[tuple[int, tuple[tuple[str, str], ...]], ...],
    expected_item_default_count: int = EXPECTED_ITEM_BLOCK_STATE_COMPONENT_COUNT,
) -> str:
    if len(block_instruments) != len(defaults):
        raise ValueError("block instrument count does not match default-state count")
    if len(item_defaults) != expected_item_default_count:
        raise ValueError(
            "unexpected Vanilla item BLOCK_STATE prototype count: "
            f"{len(item_defaults)}"
        )
    instrument_palette = tuple(dict.fromkeys(block_instruments))
    if len(instrument_palette) > 256:
        raise ValueError("note-block instrument palette exceeds u8 indices")
    palette_indices = {
        instrument: index for index, instrument in enumerate(instrument_palette)
    }
    lines = [
        "// @generated by scripts/generate-block-placement-defaults.py; do not edit manually.",
        "// Source: Vanilla 26.1 built-in block default states, note-block instruments, and item prototype BLOCK_STATE components, evaluated by the checked-in runtime probe.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-block-placement-defaults.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-block-placement-defaults.py --check",
        "",
        "#[rustfmt::skip]",
        "pub(super) const VANILLA_BLOCK_DEFAULT_STATE_IDS: &[i32] = &[",
    ]
    for start in range(0, len(defaults), 16):
        row = defaults[start : start + 16]
        lines.append("    " + ", ".join(str(state_id) for state_id in row) + ",")
    lines.extend(
        (
            "];",
            "",
            "pub(super) const VANILLA_BLOCK_NOTE_INSTRUMENT_PALETTE: &[&str] = &[",
        )
    )
    for instrument in instrument_palette:
        lines.append(f'    "{instrument}",')
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_BLOCK_NOTE_INSTRUMENT_INDICES: &[u8] = &[",
        )
    )
    for start in range(0, len(block_instruments), 32):
        row = block_instruments[start : start + 32]
        lines.append(
            "    "
            + ", ".join(str(palette_indices[instrument]) for instrument in row)
            + ","
        )
    lines.extend(
        (
            "];",
            "",
            "#[rustfmt::skip]",
            "pub(super) const VANILLA_ITEM_DEFAULT_BLOCK_STATE_PROPERTIES: &[(i32, &[(&str, &str)])] = &[",
        )
    )
    for item_id, properties in item_defaults:
        rendered = ", ".join(f'("{name}", "{value}")' for name, value in properties)
        lines.append(f"    ({item_id}, &[{rendered}]),")
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
    states, block_resources = expected_catalogs()
    facts = (
        args.facts_input.read_text()
        if args.facts_input is not None
        else runtime_facts(args)
    )
    defaults, block_instruments, item_defaults = parse_facts(
        facts, states, block_resources
    )
    generated = render_rust(defaults, block_instruments, item_defaults)
    if args.check:
        return 0 if check_output(args.output, generated) else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
