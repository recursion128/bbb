#!/usr/bin/env python3
"""Summarize open work across the coverage ledgers and the feature-gap ledger.

Informational panel for the goal.md hardest-ready selection step: for every
coverage ledger it prints the row total and the closed/deferred/open split
with each open row's non-closed units, and for docs/unsupported-features.md
it lists the top-level Todo domains. Always exits 0; gates own enforcement.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

LEDGERS = (
    "docs/packet-coverage-26.1.json",
    "docs/registry-holder-consumer-coverage-26.1.json",
    "docs/item-component-coverage-26.1.json",
    "docs/entity-coverage-26.1.json",
)
FEATURE_LEDGER = "docs/unsupported-features.md"
CLOSED_STATUSES = frozenset({"complete", "not-applicable", "not-needed"})
DEFERRED_STATUSES = frozenset({"deferred"})
ROW_LABEL_KEYS = ("resource_id", "constant_name", "class_name", "id")


def collect_statuses(value, path=""):
    """Yield (unit_path, status) for every nested string "status" field."""
    if isinstance(value, dict):
        for key, child in value.items():
            child_path = f"{path}.{key}" if path else key
            if key == "status" and isinstance(child, str):
                yield path, child
            else:
                yield from collect_statuses(child, child_path)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from collect_statuses(child, f"{path}[{index}]")


def classify_row(row):
    """Return ("closed" | "deferred" | "open", open_units) for a ledger row."""
    statuses = list(collect_statuses(row))
    if not statuses:
        return "open", [("(no status fields)", "?")]
    open_units = [
        (path or "status", status)
        for path, status in statuses
        if status not in CLOSED_STATUSES and status not in DEFERRED_STATUSES
    ]
    if open_units:
        return "open", open_units
    if any(status in DEFERRED_STATUSES for _path, status in statuses):
        return "deferred", []
    return "closed", []


def row_label(row):
    for key in ROW_LABEL_KEYS:
        value = row.get(key)
        if value:
            return str(value)
    return "<unlabeled>"


def summarize_ledger(path: Path) -> list[str]:
    entries = json.loads(path.read_text(encoding="utf-8"))["entries"]
    counts = {"closed": 0, "deferred": 0, "open": 0}
    open_rows = []
    for row in entries:
        kind, open_units = classify_row(row)
        counts[kind] += 1
        if kind == "open":
            open_rows.append((row_label(row), open_units))
    lines = [
        f"{path.name}: {len(entries)} rows"
        f" | closed {counts['closed']}"
        f" | deferred {counts['deferred']}"
        f" | open {counts['open']}"
        + ("" if counts["open"] else " (no open rows)")
    ]
    for label, units in open_rows:
        rendered = ", ".join(f"{unit}:{status}" for unit, status in units)
        lines.append(f"  open {label}: {rendered}")
    return lines


def summarize_feature_ledger(path: Path) -> list[str]:
    section = None
    todo_domains = []
    deferred_count = 0
    not_needed_count = 0
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("# "):
            section = line[2:].strip().lower()
        elif line.startswith("## "):
            heading = line[3:].strip()
            if section == "todo":
                todo_domains.append(heading)
            elif section == "deferred":
                deferred_count += 1
            elif section == "not needed":
                not_needed_count += 1
    lines = [
        f"{path.name}: {len(todo_domains)} todo domains"
        f" | {deferred_count} deferred sections"
        f" | {not_needed_count} not-needed sections"
    ]
    for domain in todo_domains:
        lines.append(f"  todo {domain}")
    return lines


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="repository root holding docs/ (default: this script's repo)",
    )
    args = parser.parse_args(argv)
    blocks = [summarize_ledger(args.repo_root / ledger) for ledger in LEDGERS]
    blocks.append(summarize_feature_ledger(args.repo_root / FEATURE_LEDGER))
    print("\n".join(line for block in blocks for line in block))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
