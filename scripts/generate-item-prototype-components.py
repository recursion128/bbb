#!/usr/bin/env python3
"""Generate Vanilla 26.1 item prototype component codec values."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import subprocess
import tempfile
from collections import defaultdict
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DOWNLOAD_ROOT = Path.home() / "Work/mc-code/downloads/26.1"
DEFAULT_JDK_ROOT = Path.home() / ".local/jdks/jdk-25.0.3+9"
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-pack/data/item_prototype_components_26_1.json"
)
PROBE_SOURCE = (
    REPO_ROOT / "scripts/vanilla-runtime/ItemPrototypeComponentsReport.java"
)
EXPECTED_ITEM_COUNT = 1506
RESOURCE_ID = re.compile(r"[a-z0-9_.-]+:[a-z0-9_./-]+")
HEADER = re.compile(
    r"version=26\.1\titems=(\d+)\tcomponents=(\d+)\tmemberships=(\d+)"
)


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


def runtime_facts(args: argparse.Namespace) -> str:
    jars = [args.client_jar, *sorted(args.libraries_root.rglob("*.jar"))]
    missing = [path for path in (args.javac, args.java, *jars[:1]) if not path.is_file()]
    if missing:
        raise FileNotFoundError(
            "missing Vanilla runtime probe input: "
            + ", ".join(str(path) for path in missing)
        )
    classpath = os.pathsep.join(str(path) for path in jars)
    with tempfile.TemporaryDirectory(
        prefix="bbb-item-prototype-components-"
    ) as directory:
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
                "ItemPrototypeComponentsReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla item prototype component probe failed:\n"
                + result.stderr.strip()
                + ("\nstdout:\n" + result.stdout.strip() if result.stdout else "")
            )
    return result.stdout


def item_id_ranges(item_ids: list[int]) -> list[list[int]]:
    if not item_ids:
        raise ValueError("component value group has no items")
    ranges = []
    start = previous = item_ids[0]
    for item_id in item_ids[1:]:
        if item_id == previous:
            raise ValueError(f"duplicate item id {item_id} in component value group")
        if item_id < previous:
            raise ValueError("component value group item ids are not sorted")
        if item_id != previous + 1:
            ranges.append([start, previous])
            start = item_id
        previous = item_id
    ranges.append([start, previous])
    return ranges


def parse_facts(source: str) -> dict[str, Any]:
    lines = source.splitlines()
    if not lines:
        raise ValueError("empty item prototype component report")
    header = HEADER.fullmatch(lines[0])
    if header is None:
        raise ValueError(f"invalid item prototype component report header: {lines[0]!r}")
    item_count, component_count, membership_count = map(int, header.groups())
    if item_count != EXPECTED_ITEM_COUNT:
        raise ValueError(
            f"expected {EXPECTED_ITEM_COUNT} item prototypes, got {item_count}"
        )
    if len(lines) <= item_count:
        raise ValueError("item prototype component report is truncated")

    item_resource_ids = []
    for expected_id, line in enumerate(lines[1 : 1 + item_count]):
        fields = line.split("\t")
        if (
            len(fields) != 3
            or fields[:2] != ["item", str(expected_id)]
            or RESOURCE_ID.fullmatch(fields[2]) is None
        ):
            raise ValueError(f"invalid item prototype identity at {expected_id}: {line!r}")
        item_resource_ids.append(fields[2])
    if len(set(item_resource_ids)) != item_count:
        raise ValueError("item prototype resource ids are not unique")

    grouped: dict[tuple[int, str], list[dict[str, Any]]] = defaultdict(list)
    seen_memberships: set[tuple[int, int]] = set()
    for line in lines[1 + item_count :]:
        fields = line.split("\t")
        if len(fields) != 5 or fields[0] != "component":
            raise ValueError(f"invalid item prototype component fact: {line!r}")
        try:
            type_id = int(fields[1])
        except ValueError as error:
            raise ValueError(f"invalid component type id: {line!r}") from error
        resource_id = fields[2]
        if type_id < 0 or RESOURCE_ID.fullmatch(resource_id) is None:
            raise ValueError(f"invalid component identity: {line!r}")
        try:
            value = json.loads(fields[3])
        except json.JSONDecodeError as error:
            raise ValueError(
                f"invalid component codec JSON for {resource_id}: {fields[3]!r}"
            ) from error
        try:
            item_ids = [int(value) for value in fields[4].split(",")]
        except ValueError as error:
            raise ValueError(f"invalid component item ids: {line!r}") from error
        if any(not 0 <= item_id < item_count for item_id in item_ids):
            raise ValueError(f"component item id outside registry: {line!r}")
        for item_id in item_ids:
            membership = (item_id, type_id)
            if membership in seen_memberships:
                raise ValueError(
                    f"duplicate component {type_id} for item {item_id}"
                )
            seen_memberships.add(membership)
        grouped[(type_id, resource_id)].append(
            {
                "value": value,
                "item_id_ranges": item_id_ranges(item_ids),
            }
        )

    resource_ids_by_type: dict[int, str] = {}
    for type_id, resource_id in grouped:
        previous = resource_ids_by_type.setdefault(type_id, resource_id)
        if previous != resource_id:
            raise ValueError(
                f"component type id {type_id} has both {previous} and {resource_id}"
            )
    if len(resource_ids_by_type) != component_count:
        raise ValueError(
            f"header declares {component_count} component types, "
            f"report contains {len(resource_ids_by_type)}"
        )
    if len(seen_memberships) != membership_count:
        raise ValueError(
            f"header declares {membership_count} memberships, "
            f"report contains {len(seen_memberships)}"
        )

    component_values = []
    for type_id in sorted(resource_ids_by_type):
        resource_id = resource_ids_by_type[type_id]
        component_values.append(
            {
                "type_id": type_id,
                "resource_id": resource_id,
                "values": grouped[(type_id, resource_id)],
            }
        )
    return {
        "vanilla_version": "26.1",
        "source": (
            "BuiltInRegistries.DATA_COMPONENT_INITIALIZERS with "
            "VanillaRegistries.createLookup and each persistent component codec"
        ),
        "item_resource_ids": item_resource_ids,
        "component_values": component_values,
    }


def render_catalog(catalog: dict[str, Any]) -> str:
    return json.dumps(
        catalog,
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
    ) + "\n"


def main() -> int:
    args = parse_args()
    source = (
        args.facts_input.read_text(encoding="utf-8")
        if args.facts_input is not None
        else runtime_facts(args)
    )
    rendered = render_catalog(parse_facts(source))
    if args.check:
        actual = (
            args.output.read_text(encoding="utf-8")
            if args.output.is_file()
            else ""
        )
        if actual == rendered:
            return 0
        print(
            "".join(
                difflib.unified_diff(
                    actual.splitlines(keepends=True),
                    rendered.splitlines(keepends=True),
                    fromfile=str(args.output),
                    tofile="generated",
                )
            ),
            end="",
        )
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
