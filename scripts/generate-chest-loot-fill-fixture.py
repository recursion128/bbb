#!/usr/bin/env python3
"""Generate the Vanilla 26.1 whole-table chest loot fill oracle."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_runtime, resolve_runtime_inputs

REPO_ROOT = Path(__file__).resolve().parents[1]
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/ChestLootFillReport.java"
PROBE_CLASS = "ChestLootFillReport"
DEFAULT_OUTPUT = REPO_ROOT / "scripts/data/chest-loot-fills-26.1.json"
ORIGIN = (0.5, 64.5, 0.5)
SEEDS = (1, 2, 7, 42, -7, 8675309, 1234567890123, -9223372036854775807)
SLOTS = 27
TABLE_COUNT = 56
TABLE_ID = re.compile(r"minecraft:chests/[a-z0-9_/]+\Z")
# One component each chest-only loot function writes, so a regenerated
# fixture keeps proving that every function is reached.
REACHED = (
    '"minecraft:stored_enchantments":',
    '"minecraft:enchantments":',
    '"minecraft:damage":',
    '"minecraft:item_name":',
    '"minecraft:suspicious_stew_effects":',
    '"minecraft:instrument":',
    '"minecraft:wind_burst":',
)


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
    with tempfile.TemporaryDirectory(prefix="bbb-chest-loot-fill-") as directory:
        subprocess.run(
            [str(args.javac), "-cp", classpath, "-d", directory, str(PROBE_SOURCE)],
            check=True,
            capture_output=True,
        )
        result = subprocess.run(
            [
                str(args.java),
                "-Xmx3G",
                "-cp",
                os.pathsep.join((directory, classpath)),
                PROBE_CLASS,
                *(repr(value) for value in ORIGIN),
                *(str(seed) for seed in SEEDS),
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla chest loot fill probe failed:\n"
                + result.stderr.strip()
                + "\nstdout:\n"
                + result.stdout.strip()
            )
        return result.stdout


def parse_report(report: str) -> dict[str, object]:
    rows = [line.split("\t") for line in report.splitlines() if line[:2] in ("H\t", "F\t", "E\t")]
    if not rows or rows[0][0] != "H":
        raise ValueError("the report has no header row")
    header = dict(field.split("=", 1) for field in rows[0][1:])
    expected_header = {
        "version": "26.1",
        "tables": str(TABLE_COUNT),
        "seeds": ",".join(str(seed) for seed in SEEDS),
        "origin": ",".join(repr(value) for value in ORIGIN),
        "slots": str(SLOTS),
    }
    if header != expected_header:
        raise ValueError(f"unexpected header {header}")
    tables: dict[str, dict[str, dict[str, str]]] = {}
    pending: dict[tuple[str, str], dict[str, str]] = {}
    for row in rows[1:]:
        kind, table, seed = row[0], row[1], row[2]
        if TABLE_ID.match(table) is None or str(int(seed)) != seed or int(seed) not in SEEDS:
            raise ValueError(f"malformed row {row}")
        fill = pending.setdefault((table, seed), {})
        if kind == "F":
            if len(row) != 5:
                raise ValueError(f"malformed slot row {row}")
            slot = int(row[3])
            if not 0 <= slot < SLOTS or str(slot) in fill or not row[4].startswith("{"):
                raise ValueError(f"malformed slot row {row}")
            fill[str(slot)] = row[4]
        elif kind == "E":
            if len(row) != 4 or int(row[3]) != len(fill):
                raise ValueError(f"slot count mismatch at {row}")
            seeds = tables.setdefault(table, {})
            if seed in seeds:
                raise ValueError(f"duplicate fill {table} {seed}")
            seeds[seed] = pending.pop((table, seed))
        else:
            raise ValueError(f"unexpected row {row}")
    if pending:
        raise ValueError(f"fills without an end row: {sorted(pending)}")
    if len(tables) != TABLE_COUNT or any(len(seeds) != len(SEEDS) for seeds in tables.values()):
        raise ValueError("the report does not cover every table under every seed")
    written = [stack for seeds in tables.values() for fill in seeds.values() for stack in fill.values()]
    for marker in REACHED:
        if not any(marker in stack for stack in written):
            raise ValueError(f"no filled stack carries {marker}")
    return {
        "origin": list(ORIGIN),
        "seeds": [str(seed) for seed in SEEDS],
        "slots": SLOTS,
        "tables": tables,
        "version": "26.1",
    }


def encode(artifact: dict[str, object]) -> str:
    return json.dumps(artifact, indent=2, sort_keys=True) + "\n"


def main() -> int:
    args = parse_args()
    report = args.report_input.read_text() if args.report_input else runtime_report(args)
    expected = encode(parse_report(report))
    if args.check:
        actual = args.output.read_text() if args.output.is_file() else ""
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
    args.output.write_text(expected)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
