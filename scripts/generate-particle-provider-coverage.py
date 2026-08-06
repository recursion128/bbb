#!/usr/bin/env python3
"""Generate the checked-in vanilla 26.1 particle provider coverage ledger.

Row identity — every `ParticleProvider` plus the handful of particles vanilla
spawns straight from client code — the particle each row builds, and the five
behavioural facts are all derived from the Vanilla sources by
`particle_provider_catalog_extractor`, so no hand-written table can drift away
from them.  The overrides file records the audited fingerprint of that derived
table plus the evidence sets each column cites; a Vanilla change or a sharper
derivation therefore fails generation until the new facts are audited.

A cell is `not-applicable` exactly when Vanilla does not exhibit the behaviour,
and carries the derivation that decided it.  A cell is `complete` when Vanilla
does exhibit it and the renderer's descriptor table names the row.  Any other
combination has to be classified by hand in the overrides file, which is what
keeps a newly-reachable behaviour from being absorbed silently.

The renderer's own `has_physics` flags are cross-checked against the derived
`hasPhysics` for every provider a descriptor arm names; a disagreement fails
generation rather than being recorded.
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

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root

from particle_provider_catalog_extractor import (
    ParticleFacts,
    ParticleSources,
    extract_particle_facts,
)


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DESCRIPTORS = REPO_ROOT / "crates/bbb-renderer/src/particles/descriptors.rs"
DEFAULT_OVERRIDES = REPO_ROOT / "scripts/data/particle-provider-coverage-overrides-26.1.json"
DEFAULT_OUTPUT = REPO_ROOT / "docs/evidence/vanilla-26.1/particle-provider-coverage.json"

CELL_CATEGORIES = (
    "renderer binding",
    "collision",
    "player-coupled",
    "sounds",
    "conditional-removal",
)
ALLOWED_STATUSES = frozenset({"complete", "partial", "deferred", "not-applicable"})

# Each descriptor arm of `ParticleDescriptor::for_particle`. The provider name
# also appears in helper constructors outside the match, so binding is read
# from the whole file and only the physics cross-check needs the arms.
DESCRIPTOR_ARM = "=> Self {"
PROVIDER_LITERAL_RE = re.compile(r'"([A-Z][A-Za-z0-9_]*\.[A-Za-z0-9_]*Provider)"')
HAS_PHYSICS_RE = re.compile(r"has_physics: (true|false)")


@dataclass(frozen=True)
class RendererBinding:
    """What the renderer's descriptor table says about the vanilla particles."""

    text: str
    providers: frozenset[str]
    has_physics: dict[str, bool]

    def binds(self, name: str) -> bool:
        """Whether the descriptor table names this row.

        Providers appear as `"AshParticle.Provider"`, code-spawned particles as
        their own class name, so one quoted-name test covers both.
        """
        return f'"{name}"' in self.text


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--descriptors", type=Path, default=DEFAULT_DESCRIPTORS)
    parser.add_argument("--overrides", type=Path, default=DEFAULT_OVERRIDES)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    add_vanilla_root(parser)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


def strict_json(path: Path) -> object:
    def strict_object(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError(f"{path}: duplicate JSON key {key!r}")
            value[key] = item
        return value

    return json.loads(path.read_text(), object_pairs_hook=strict_object)


def load_overrides(path: Path) -> dict[str, object]:
    value = strict_json(path)
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError(f"{path}: invalid particle provider coverage override schema")
    return value


def brace_block(text: str, opening: int) -> str:
    """The `{...}` block starting at the brace at or after `opening`."""
    start = text.index("{", opening)
    depth = 0
    for index in range(start, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[start : index + 1]
    raise ValueError("unbalanced descriptor literal")


def parse_renderer_binding(path: Path) -> RendererBinding:
    text = path.read_text()
    providers = set(PROVIDER_LITERAL_RE.findall(text))
    if not providers:
        raise ValueError(f"{path}: no vanilla provider names found")
    has_physics: dict[str, bool] = {}
    index = text.find(DESCRIPTOR_ARM)
    while index >= 0:
        block = brace_block(text, index)
        physics = HAS_PHYSICS_RE.search(block)
        if physics is not None:
            for provider in PROVIDER_LITERAL_RE.findall(block):
                claimed = physics.group(1) == "true"
                if has_physics.setdefault(provider, claimed) != claimed:
                    raise ValueError(
                        f"{path}: descriptor arms disagree on has_physics for {provider}"
                    )
        index = text.find(DESCRIPTOR_ARM, index + len(block))
    return RendererBinding(text, frozenset(providers), has_physics)


def cross_check_physics(facts: list[ParticleFacts], binding: RendererBinding) -> None:
    """Fail when the renderer's physics flag contradicts the derived Vanilla one."""
    disagreements = []
    for row in facts:
        claimed = binding.has_physics.get(row.name)
        if claimed is None:
            continue
        derived = row.collision.reason != "hasPhysics=false"
        if claimed != derived:
            disagreements.append(f"{row.name}: renderer {claimed}, vanilla {derived}")
    if disagreements:
        raise ValueError(
            "descriptor has_physics disagrees with Vanilla:\n  " + "\n  ".join(disagreements)
        )


def catalog_sha256(facts: list[ParticleFacts]) -> str:
    """Fingerprint of the derived table, so new Vanilla facts force an audit."""
    lines = []
    for row in facts:
        cells = "\t".join(
            f"{name}={int(fact.present)}:{fact.reason}" for name, fact in row.facts().items()
        )
        lines.append(f"{row.name}\t{row.spawn}\t{row.particle}\t{cells}")
    return hashlib.sha256(("\n".join(lines) + "\n").encode()).hexdigest()


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


def validate_override_cell(
    cell: object, evidence_sets: dict[str, dict[str, list[str]]], context: str
) -> dict[str, str]:
    """A hand-classified cell: the only place a status is not derived.

    Vanilla spawns a few particles the renderer owns without naming them in the
    descriptor table, so `complete` is reachable here — but only against the
    same implementation-and-tests bar a derived `complete` clears.
    """
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
                f"{context}: complete cell requires implementation and test evidence"
            )
        if gap is not None:
            raise ValueError(f"{context}: complete cell must not carry a gap")
        return {"status": status, "evidence_ref": evidence_ref}
    if not (isinstance(gap, str) and gap.startswith("docs/unsupported-features.md#")):
        raise ValueError(f"{context}: open cell requires an unsupported-features gap")
    if not evidence["vanilla"]:
        raise ValueError(f"{context}: open cell requires vanilla evidence")
    return {"status": status, "evidence_ref": evidence_ref, "gap": gap}


def derive_cell(
    category: str,
    row: ParticleFacts,
    bound: bool,
    evidence_ref: str,
    evidence_sets: dict[str, dict[str, list[str]]],
) -> dict[str, str] | None:
    """The derived cell, or None when the pair needs a hand classification."""
    fact = row.facts()[category]
    if not fact.present:
        return {
            "status": "not-applicable",
            "evidence_ref": evidence_ref,
            "gap": fact.reason,
        }
    if not bound:
        return None
    evidence = evidence_sets[evidence_ref]
    if not evidence["implementation"] or not evidence["tests"]:
        raise ValueError(
            f"{row.name}/{category}: complete cell requires implementation and test evidence"
        )
    return {"status": "complete", "evidence_ref": evidence_ref}


def render_ledger(
    facts: list[ParticleFacts],
    binding: RendererBinding,
    overrides: dict[str, object],
    source_root: Path,
) -> str:
    expected_hash = overrides.get("catalog_sha256")
    actual_hash = catalog_sha256(facts)
    if expected_hash != actual_hash:
        raise ValueError(
            "derived particle provider facts changed; audit them and update catalog_sha256 "
            f"({expected_hash!r} != {actual_hash!r})"
        )
    evidence_sets = validate_evidence_sets(overrides.get("evidence_sets"), source_root)
    column_evidence = overrides.get("column_evidence")
    if not isinstance(column_evidence, dict) or set(column_evidence) != set(CELL_CATEGORIES):
        raise ValueError("column_evidence must name an evidence set for every cell category")
    unknown_refs = set(column_evidence.values()) - set(evidence_sets)
    if unknown_refs:
        raise ValueError(f"column_evidence names unknown evidence sets: {sorted(unknown_refs)}")

    provider_overrides = overrides.get("row_overrides", {})
    if not isinstance(provider_overrides, dict):
        raise ValueError("row_overrides must be an object")
    known = {row.name for row in facts}
    unknown_providers = set(provider_overrides) - known
    if unknown_providers:
        raise ValueError(f"unknown row overrides: {sorted(unknown_providers)}")

    unknown_bindings = binding.providers - known
    if unknown_bindings:
        raise ValueError(
            f"renderer names providers Vanilla 26.1 does not declare: {sorted(unknown_bindings)}"
        )

    rows = []
    for row in facts:
        row_overrides = provider_overrides.get(row.name, {})
        if not isinstance(row_overrides, dict):
            raise ValueError(f"{row.name}: override must be an object")
        unknown_cells = set(row_overrides) - set(CELL_CATEGORIES)
        if unknown_cells:
            raise ValueError(f"{row.name}: unknown cell categories {sorted(unknown_cells)}")
        bound = binding.binds(row.name)
        cells = {}
        for category in CELL_CATEGORIES:
            context = f"{row.name}/{category}"
            derived = derive_cell(
                category, row, bound, column_evidence[category], evidence_sets
            )
            override = row_overrides.get(category)
            if derived is None:
                if override is None:
                    raise ValueError(
                        f"{context}: Vanilla exhibits this behaviour but the renderer names no "
                        "descriptor for the particle; classify the cell in the overrides file"
                    )
                cells[category] = validate_override_cell(override, evidence_sets, context)
                continue
            if override is not None:
                raise ValueError(f"{context}: derived cell must not be overridden")
            cells[category] = derived
        rows.append(
            {
                "name": row.name,
                "particle": row.particle,
                "spawn": row.spawn,
                "renderer_descriptor": bound,
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
        "generated_by": "scripts/generate-particle-provider-coverage.py",
        "update_command": (
            "scripts/vanilla-source-tools.sh scripts/generate-particle-provider-coverage.py"
        ),
        "check_command": (
            "scripts/vanilla-source-tools.sh scripts/generate-particle-provider-coverage.py"
            " --check"
        ),
        "catalog_source": "vanilla:net/minecraft/client/particle/#ParticleProvider",
        "catalog_sha256": actual_hash,
        "coverage_overrides": "scripts/data/particle-provider-coverage-overrides-26.1.json",
        "renderer_source": "crates/bbb-renderer/src/particles/descriptors.rs#for_particle",
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
    sources = ParticleSources.load(args.source_root)
    facts = extract_particle_facts(sources)
    binding = parse_renderer_binding(args.descriptors)
    cross_check_physics(facts, binding)
    overrides = load_overrides(args.overrides)
    generated = render_ledger(facts, binding, overrides, args.source_root)
    if args.check:
        if check_output(args.output, generated):
            print(f"particle provider coverage: {args.output} is current")
            return 0
        print(f"particle provider coverage: {args.output} is stale", file=sys.stderr)
        return 1
    args.output.write_text(generated)
    print(f"particle provider coverage: wrote {len(facts)} rows to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
