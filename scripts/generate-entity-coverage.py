#!/usr/bin/env python3
"""Generate the checked-in vanilla 26.1 entity coverage ledger.

Entity identity and order are parsed from the generated Rust catalog so this
tool never owns a second entity name/id table.  The overrides file records the
audited catalog fingerprint and shared coverage profiles; changing the catalog
therefore requires an explicit new audit before generation succeeds.
"""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CATALOG = REPO_ROOT / "crates/bbb-protocol/src/entity_types.rs"
DEFAULT_OVERRIDES = REPO_ROOT / "scripts/data/entity-coverage-overrides-26.1.json"
DEFAULT_OUTPUT = REPO_ROOT / "docs/evidence/vanilla-26.1/entity-coverage.json"
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"

CELL_CATEGORIES = (
    "spawn/default",
    "metadata/schema",
    "transform/interpolation",
    "client tick/movement",
    "dimensions/collision/pick",
    "model/texture/layers",
    "animation",
    "equipment/passenger/leash",
    "event/particle/audio",
    "save/debug",
)
ALLOWED_STATUSES = frozenset(
    {"complete", "partial", "deferred", "not-applicable"}
)
ENTRY_PATTERN = re.compile(
    r"EntityTypeCatalogEntry \{\s*"
    r"id: (?P<id>\d+),\s*"
    r'constant_name: "(?P<constant>[A-Z0-9_]+)",\s*'
    r'resource_id: "(?P<resource>minecraft:[a-z0-9_]+)",',
    re.MULTILINE,
)


@dataclass(frozen=True)
class EntityType:
    id: int
    constant_name: str
    resource_id: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--overrides", type=Path, default=DEFAULT_OVERRIDES)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def strict_json(path: Path) -> object:
    def strict_object(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError(f"{path}: duplicate JSON key {key!r}")
            value[key] = item
        return value

    return json.loads(path.read_text(), object_pairs_hook=strict_object)


def load_entity_catalog(path: Path) -> list[EntityType]:
    source = path.read_text()
    marker = "pub const VANILLA_ENTITY_TYPES: &[EntityTypeCatalogEntry] = &["
    try:
        entries_source = source.split(marker, 1)[1]
    except IndexError as error:
        raise ValueError(f"{path}: missing VANILLA_ENTITY_TYPES catalog") from error
    entries_source = entries_source.split("];", 1)[0]
    entries = [
        EntityType(
            id=int(match.group("id")),
            constant_name=match.group("constant"),
            resource_id=match.group("resource"),
        )
        for match in ENTRY_PATTERN.finditer(entries_source)
    ]
    if not entries:
        raise ValueError(f"{path}: VANILLA_ENTITY_TYPES has no parsed entries")
    for expected_id, entry in enumerate(entries):
        if entry.id != expected_id:
            raise ValueError(
                f"{path}: entity id/order mismatch at {entry.resource_id}: "
                f"expected {expected_id}, got {entry.id}"
            )
    resources = {entry.resource_id for entry in entries}
    constants = {entry.constant_name for entry in entries}
    if len(resources) != len(entries) or len(constants) != len(entries):
        raise ValueError(f"{path}: duplicate entity resource or constant")
    return entries


def catalog_sha256(entries: list[EntityType]) -> str:
    canonical = "\n".join(
        f"{entry.id}\t{entry.constant_name}\t{entry.resource_id}" for entry in entries
    )
    return hashlib.sha256((canonical + "\n").encode()).hexdigest()


def load_overrides(path: Path) -> dict[str, object]:
    value = strict_json(path)
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError(f"{path}: invalid entity coverage override schema")
    return value


def validate_locator(locator: str, source_root: Path, context: str) -> None:
    if "#" not in locator:
        raise ValueError(f"{context}: malformed evidence locator {locator!r}")
    path_text, symbol = locator.split("#", 1)
    if not symbol:
        raise ValueError(f"{context}: evidence locator lacks symbol {locator!r}")
    if path_text.startswith("vanilla:"):
        root = source_root
        path_text = path_text.removeprefix("vanilla:")
    else:
        root = REPO_ROOT
    path = Path(path_text)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"{context}: evidence path must be relative {locator!r}")
    source_path = root / path
    if not source_path.is_file():
        raise ValueError(f"{context}: evidence file does not exist: {locator}")
    if symbol not in source_path.read_text():
        raise ValueError(f"{context}: evidence symbol not found: {locator}")


def validate_evidence_sets(
    evidence_sets: object, source_root: Path
) -> dict[str, dict[str, list[str]]]:
    if not isinstance(evidence_sets, dict) or not evidence_sets:
        raise ValueError("evidence_sets must be a non-empty object")
    validated: dict[str, dict[str, list[str]]] = {}
    for name, evidence in evidence_sets.items():
        if not isinstance(name, str) or not isinstance(evidence, dict):
            raise ValueError("invalid evidence set")
        unknown = set(evidence) - {"implementation", "tests", "vanilla"}
        if unknown:
            raise ValueError(f"evidence set {name}: unknown keys {sorted(unknown)}")
        normalized: dict[str, list[str]] = {}
        for kind in ("implementation", "tests", "vanilla"):
            locators = evidence.get(kind, [])
            if not isinstance(locators, list) or not all(
                isinstance(locator, str) and locator for locator in locators
            ):
                raise ValueError(f"evidence set {name}/{kind}: invalid locators")
            for locator in locators:
                validate_locator(locator, source_root, f"evidence set {name}/{kind}")
            normalized[kind] = locators
        validated[name] = normalized
    return validated


def validate_cell(
    cell: object,
    evidence_sets: dict[str, dict[str, list[str]]],
    context: str,
) -> dict[str, str]:
    if not isinstance(cell, dict):
        raise ValueError(f"{context}: cell must be an object")
    unknown = set(cell) - {"status", "evidence_ref", "gap"}
    if unknown:
        raise ValueError(f"{context}: unknown cell keys {sorted(unknown)}")
    status = cell.get("status")
    evidence_ref = cell.get("evidence_ref")
    gap = cell.get("gap")
    if status not in ALLOWED_STATUSES:
        raise ValueError(f"{context}: invalid status {status!r}")
    if not isinstance(evidence_ref, str) or evidence_ref not in evidence_sets:
        raise ValueError(f"{context}: unknown evidence_ref {evidence_ref!r}")
    evidence = evidence_sets[evidence_ref]
    if status == "complete":
        if not evidence["implementation"] or not evidence["tests"]:
            raise ValueError(
                f"{context}: complete cell requires implementation and semantic test evidence"
            )
        if gap is not None:
            raise ValueError(f"{context}: complete cell must not carry a gap")
        return {"status": status, "evidence_ref": evidence_ref}
    if not (
        isinstance(gap, str) and gap.startswith("docs/unsupported-features.md#")
    ):
        raise ValueError(f"{context}: non-complete cell requires an unsupported gap")
    if not evidence["vanilla"]:
        raise ValueError(f"{context}: non-complete cell requires vanilla evidence")
    return {"status": status, "evidence_ref": evidence_ref, "gap": gap}


def render_ledger(
    entries: list[EntityType], overrides: dict[str, object], source_root: Path
) -> str:
    expected_hash = overrides.get("catalog_sha256")
    actual_hash = catalog_sha256(entries)
    if expected_hash != actual_hash:
        raise ValueError(
            "entity catalog changed; audit the new catalog and update catalog_sha256 "
            f"({expected_hash!r} != {actual_hash!r})"
        )
    evidence_sets = validate_evidence_sets(overrides.get("evidence_sets"), source_root)
    base_cells = overrides.get("base_cells")
    if not isinstance(base_cells, dict) or set(base_cells) != set(CELL_CATEGORIES):
        raise ValueError("base_cells must classify exactly the ten cell categories")
    entity_overrides = overrides.get("entity_overrides", {})
    if not isinstance(entity_overrides, dict):
        raise ValueError("entity_overrides must be an object")
    known_resources = {entry.resource_id for entry in entries}
    unknown_entities = set(entity_overrides) - known_resources
    if unknown_entities:
        raise ValueError(f"unknown entity overrides: {sorted(unknown_entities)}")

    rows = []
    for entry in entries:
        row_overrides = entity_overrides.get(entry.resource_id, {})
        if not isinstance(row_overrides, dict):
            raise ValueError(f"{entry.resource_id}: override must be an object")
        unknown_cells = set(row_overrides) - set(CELL_CATEGORIES)
        if unknown_cells:
            raise ValueError(
                f"{entry.resource_id}: unknown cell categories {sorted(unknown_cells)}"
            )
        cells = {}
        for category in CELL_CATEGORIES:
            raw_cell = row_overrides.get(category, base_cells[category])
            cells[category] = validate_cell(
                raw_cell, evidence_sets, f"{entry.resource_id}/{category}"
            )
        rows.append(
            {
                "id": entry.id,
                "constant_name": entry.constant_name,
                "resource_id": entry.resource_id,
                "coverage": cells,
            }
        )

    uncertainties = overrides.get("uncertainties", [])
    if not isinstance(uncertainties, list) or not all(
        isinstance(item, str) and item for item in uncertainties
    ):
        raise ValueError("uncertainties must be a list of non-empty strings")
    value = {
        "schema_version": 1,
        "vanilla_version": "26.1",
        "generated_by": "scripts/generate-entity-coverage.py",
        "update_command": (
            "scripts/vanilla-source-tools.sh scripts/generate-entity-coverage.py"
        ),
        "check_command": (
            "scripts/vanilla-source-tools.sh scripts/generate-entity-coverage.py --check"
        ),
        "catalog_source": "crates/bbb-protocol/src/entity_types.rs#VANILLA_ENTITY_TYPES",
        "catalog_sha256": actual_hash,
        "coverage_overrides": "scripts/data/entity-coverage-overrides-26.1.json",
        "cell_categories": list(CELL_CATEGORIES),
        "allowed_statuses": sorted(ALLOWED_STATUSES),
        "baseline_policy": overrides.get("baseline_policy"),
        "uncertainties": uncertainties,
        "evidence_sets": evidence_sets,
        "entries": rows,
    }
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


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
    entries = load_entity_catalog(args.catalog)
    overrides = load_overrides(args.overrides)
    generated = render_ledger(entries, overrides, args.source_root)
    if args.check:
        return 0 if check_output(args.output, generated) else 1
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
