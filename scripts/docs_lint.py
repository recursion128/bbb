#!/usr/bin/env python3
"""Cross-platform repository documentation, ledger, and generated-fact lint."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

HISTORICAL_SNAPSHOT_RE = re.compile(
    r"capture `[0-9a-f]{7,40}`"
    r"|gated (?:commit|evidence) `[0-9a-f]{7,40}`"
    r"|最后 gated commit"
    r"|`[0-9a-f]{40}`"
    r"|(?:HEAD|commit|基线|build-info)[^`\n]{0,24}`[0-9a-f]{7,40}`"
    r"|\b20\d{2}-\d{2}-\d{2}[T ]\d{2}:\d{2}"
    r"|(?:handoff|接手|snapshot|快照)[^\n]{0,40}\b20\d{2}-\d{2}-\d{2}\b"
    r"|\bhandoff-state\.json\b"
    r"|接手状态"
    r"|[A-Za-z]:\\bbb-cache\\[^\s`')]*20\d{6}"
    r"|/var/tmp/bbb-[^` )]+"
    r"|/tmp/bbb-[^` )]+/(?:report|evidence)"
)
DEFERRED_MARKER_RE = re.compile(r"(?:^|[^A-Za-z0-9_-])deferred(?:[^A-Za-z0-9_-]|$)", re.IGNORECASE)


def _matching_lines(path: Path, rel_label: str, pattern: re.Pattern[str]) -> list[str]:
    hits: list[str] = []
    if not path.is_file():
        return hits
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if pattern.search(line):
            hits.append(f"{rel_label}:{lineno}:{line}")
    return hits


def _parse_ledger_items(
    path: Path, rel_label: str
) -> tuple[list[str], list[str], list[str], list[str]]:
    nodomain: list[str] = []
    misplaced: list[str] = []
    missing_reopen: list[str] = []
    missing_evidence: list[str] = []

    section = ""
    domain = ""
    item = ""
    item_line = 0
    item_section = ""

    def flush_item() -> None:
        nonlocal item, item_line, item_section
        if not item:
            return
        has_deferred = bool(DEFERRED_MARKER_RE.search(item))
        if item_section == "not needed" and has_deferred:
            misplaced.append(f"{item_line}:{item}")
        if has_deferred and "Reopen when " not in item:
            missing_reopen.append(f"{item_line}:{item}")
        if (
            item_section == "not needed"
            and "vanilla" not in item.lower()
            and "`" not in item
        ):
            missing_evidence.append(f"{item_line}:{item}")
        item = ""
        item_line = 0
        item_section = ""

    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if line.startswith("# "):
            flush_item()
            section = line[2:].lower()
            domain = ""
            continue
        if line.startswith("## "):
            flush_item()
            domain = line[3:]
            continue
        if section in ("todo", "not needed") and line.startswith("- "):
            flush_item()
            item = line
            item_line = lineno
            item_section = section
            if not domain:
                nodomain.append(f"{rel_label}:{lineno}:{line}")
            continue
        if item and line.startswith("  "):
            item = f"{item} {line}"
            continue
        if item and line.strip():
            flush_item()

    flush_item()
    return nodomain, misplaced, missing_reopen, missing_evidence


OFFLINE_NO_VANILLA_GENERATORS: tuple[str, ...] = (
    "generate-block-property-strides.py",
    "generate-item-prototype-components-table.py",
)

FAST_VANILLA_GENERATORS: tuple[str, ...] = (
    "generate-biome-catalog.py",
    "generate-block-registry-catalog.py",
    "generate-bootstrap-catalog.py",
    "generate-client-options-catalog.py",
    "generate-command-argument-catalog.py",
    "generate-core-registry-catalog.py",
    "generate-creative-mode-tab-catalog.py",
    "generate-entity-default-attributes.py",
    "generate-entity-metadata-schema.py",
    "generate-entity-renderer-catalog.py",
    "generate-environment-attribute-catalog.py",
    "generate-furnace-fuel-catalog.py",
    "generate-gameplay-id-catalog.py",
    "generate-item-membership-catalog.py",
    "generate-item-registry-catalog.py",
    "generate-jukebox-song-catalog.py",
    "generate-key-mapping-catalog.py",
    "generate-menu-screen-catalog.py",
    "generate-packet-catalog.py",
    "generate-server-block-sound-facts.py",
    "generate-server-entity-runtime-facts.py",
    "generate-server-natural-spawn-facts.py",
    "generate-sound-event-catalog.py",
)

SLOW_VANILLA_GENERATORS: tuple[str, ...] = (
    "generate-block-membership-catalog.py",
    "generate-vanilla-datapack-snapshot.py",
)

JVM_RUNTIME_GENERATORS: tuple[str, ...] = (
    "generate-block-placement-defaults.py",
    "generate-block-property-facts.py",
    "generate-block-state-collision-shapes.py",
    "generate-block-state-destroy-facts.py",
    "generate-block-state-fluid-replaceability.py",
    "generate-block-state-ignited-by-lava.py",
    "generate-block-state-mirrors.py",
    "generate-block-state-outline-shapes.py",
    "generate-block-state-path-facts.py",
    "generate-block-state-random-ticks.py",
    "generate-block-state-render-facts.py",
    "generate-block-state-rotations.py",
    "generate-block-state-support-facts.py",
    "generate-chest-loot-fill-fixture.py",
    "generate-command-tree-catalog.py",
    "generate-entity-models.py",
    "generate-item-prototype-components.py",
    "generate-noise-overworld-light-fixture.py",
    "generate-static-registry-catalog.py",
)


def _run_python_script(
    script_dir: Path,
    repo_root: Path,
    script: Path,
    *args: str,
    capture: bool = False,
) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    existing_pp = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = (
        f"{script_dir}{os.pathsep}{existing_pp}" if existing_pp else str(script_dir)
    )
    env.setdefault("PYTHONDONTWRITEBYTECODE", "1")
    return subprocess.run(
        [sys.executable, str(script), *args],
        cwd=repo_root,
        env=env,
        check=False,
        text=True,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.STDOUT if capture else None,
    )


def _generator_check_args(generator_name: str, vanilla_root: Path | None) -> list[str]:
    if generator_name == "generate-entity-metadata-schema.py":
        assert vanilla_root is not None
        return ["check", "--vanilla-root", str(vanilla_root)]
    if vanilla_root is not None:
        return ["--vanilla-root", str(vanilla_root), "--check"]
    return ["--check"]


def run_docs_lint(
    repo_root: Path,
    *,
    skip_vanilla: bool = False,
    all_generators: bool = False,
) -> int:
    script_dir = Path(__file__).resolve().parent
    if str(script_dir) not in sys.path:
        sys.path.insert(0, str(script_dir))

    client_ledger = repo_root / "docs" / "unsupported-client-features.md"
    server_ledger = repo_root / "docs" / "unsupported-server-features.md"

    failed = False

    docs_dir = repo_root / "docs"
    historical_docs: list[str] = []
    if docs_dir.is_dir():
        for path in sorted(docs_dir.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(repo_root).as_posix()
            if rel.startswith("docs/archive/") or path.name == "goal-archive.md":
                historical_docs.append(rel)
    if historical_docs:
        print(
            "docs lint: completed-history archive files are not allowed; use Git history:",
            file=sys.stderr,
        )
        print("\n".join(historical_docs), file=sys.stderr)
        failed = True

    snapshot_targets: list[tuple[Path, str]] = [
        (client_ledger, "docs/unsupported-client-features.md"),
        (server_ledger, "docs/unsupported-server-features.md"),
        (repo_root / "docs" / "README.md", "docs/README.md"),
    ]
    wiki_dir = repo_root / "docs" / "wiki"
    if wiki_dir.is_dir():
        for md in sorted(wiki_dir.rglob("*.md")):
            snapshot_targets.append((md, md.relative_to(repo_root).as_posix()))
    historical_snapshots: list[str] = []
    for target_path, rel_label in snapshot_targets:
        historical_snapshots.extend(
            _matching_lines(target_path, rel_label, HISTORICAL_SNAPSHOT_RE)
        )
    if historical_snapshots:
        print(
            "docs lint: historical receipt snapshots are not allowed in Wiki pages:",
            file=sys.stderr,
        )
        print("\n".join(historical_snapshots), file=sys.stderr)
        print(
            "docs lint fix: keep current contracts and evidence links; use Git for completed runs",
            file=sys.stderr,
        )
        failed = True

    legacy_deferred_re = re.compile(r"^# Deferred$")
    legacy_deferred = _matching_lines(
        client_ledger, "docs/unsupported-client-features.md", legacy_deferred_re
    ) + _matching_lines(
        server_ledger, "docs/unsupported-server-features.md", legacy_deferred_re
    )
    if legacy_deferred:
        print("feature ledger lint: top-level Deferred sections are not allowed:", file=sys.stderr)
        print("\n".join(legacy_deferred), file=sys.stderr)
        print(
            "feature ledger lint fix: keep the item under its owning Todo domain "
            "and mark it `deferred` with an explicit 'Reopen when ...' condition",
            file=sys.stderr,
        )
        failed = True

    c_nodom, c_mis, c_reopen, c_ev = _parse_ledger_items(
        client_ledger, str(client_ledger)
    )
    s_nodom, s_mis, s_reopen, s_ev = _parse_ledger_items(
        server_ledger, str(server_ledger)
    )

    items_outside_domain = c_nodom + s_nodom
    if items_outside_domain:
        print(
            "feature ledger lint: every # Todo / # Not needed item "
            "must sit under a ## domain heading:",
            file=sys.stderr,
        )
        print("\n".join(items_outside_domain), file=sys.stderr)
        failed = True

    misplaced_deferred = c_mis + s_mis
    if misplaced_deferred:
        print(
            "feature ledger lint: deferred items must stay under their owning # Todo domain, "
            "not # Not needed:",
            file=sys.stderr,
        )
        print("\n".join(misplaced_deferred), file=sys.stderr)
        failed = True

    deferred_without_reopen = c_reopen + s_reopen
    if deferred_without_reopen:
        print(
            "feature ledger lint: deferred items need an explicit 'Reopen when ...' condition:",
            file=sys.stderr,
        )
        print("\n".join(deferred_without_reopen), file=sys.stderr)
        failed = True

    not_needed_without_evidence = c_ev + s_ev
    if not_needed_without_evidence:
        print(
            "feature ledger lint: # Not needed items need a Vanilla statement "
            "or source-code anchor:",
            file=sys.stderr,
        )
        print("\n".join(not_needed_without_evidence), file=sys.stderr)
        failed = True

    if failed:
        return 1

    vanilla_root: Path | None = None
    if not skip_vanilla:
        from bbb_tools.vanilla.errors import VanillaError
        from bbb_tools.vanilla.workspace import VanillaWorkspace

        try:
            vanilla_root = VanillaWorkspace.resolve("26.1").require_prepared().root
        except VanillaError as error:
            print(f"vanilla {error.kind}: {error}", file=sys.stderr)
            failed = True

    generator_tasks: list[tuple[str, Path, list[str]]] = []
    for generator_name in OFFLINE_NO_VANILLA_GENERATORS:
        generator = repo_root / "scripts" / generator_name
        if generator.is_file():
            generator_tasks.append(
                (generator_name, generator, _generator_check_args(generator_name, None))
            )

    jvm_generator_tasks: list[tuple[str, Path, list[str]]] = []
    if vanilla_root is not None:
        selected_vanilla = (
            FAST_VANILLA_GENERATORS + SLOW_VANILLA_GENERATORS
            if all_generators
            else FAST_VANILLA_GENERATORS
        )
        for generator_name in selected_vanilla:
            generator = repo_root / "scripts" / generator_name
            if generator.is_file():
                generator_tasks.append(
                    (
                        generator_name,
                        generator,
                        _generator_check_args(generator_name, vanilla_root),
                    )
                )
        if all_generators:
            for generator_name in JVM_RUNTIME_GENERATORS:
                generator = repo_root / "scripts" / generator_name
                if generator.is_file():
                    jvm_generator_tasks.append(
                        (
                            generator_name,
                            generator,
                            _generator_check_args(generator_name, vanilla_root),
                        )
                    )
    else:
        print(
            "docs lint: Vanilla-source generator exact-output checks skipped "
            "(Vanilla tree absent or --skip-vanilla)"
        )

    ratchet = repo_root / "scripts" / "source_file_size_ratchet.py"
    ratchet_task = [("ratchet", ratchet, ["check"])] if ratchet.is_file() else []
    total_subprocs = len(jvm_generator_tasks) + len(generator_tasks) + len(ratchet_task)
    batch_failed: dict[str, bool] = {"offline": False, "JVM runtime": False}
    ratchet_proc: subprocess.CompletedProcess[str] | None = None

    def run_wiki_checks() -> bool:
        ok = True
        gate_sync_errors = check_gate_docs_sync(repo_root)
        if gate_sync_errors:
            print("\n".join(gate_sync_errors), file=sys.stderr)
            ok = False
        from docs_wiki_lint import WikiLint

        if WikiLint(repo_root, check_vanilla=not skip_vanilla).run() != 0:
            ok = False
        return ok

    # Schedule long-pole JVM generators first and overlap offline generators,
    # size ratchet, and WikiLint in one bounded pool instead of serial waves.
    if total_subprocs:
        workers = min(12, total_subprocs, max(4, os.cpu_count() or 4))
        with ThreadPoolExecutor(max_workers=workers) as pool:
            futures = {}
            for kind, items in (
                ("JVM runtime", jvm_generator_tasks),
                ("offline", generator_tasks),
                ("ratchet", ratchet_task),
            ):
                for name, script, gen_args in items:
                    fut = pool.submit(
                        _run_python_script,
                        script_dir,
                        repo_root,
                        script,
                        *gen_args,
                        capture=True,
                    )
                    futures[fut] = (kind, name)

            if not run_wiki_checks():
                failed = True

            for future in as_completed(futures):
                kind, name = futures[future]
                proc = future.result()
                if kind == "ratchet":
                    ratchet_proc = proc
                    continue
                if proc.returncode != 0:
                    if proc.stdout:
                        sys.stderr.write(proc.stdout)
                        if not proc.stdout.endswith("\n"):
                            sys.stderr.write("\n")
                    print(f"docs lint: generator check failed: {name}", file=sys.stderr)
                    batch_failed[kind] = True
                    failed = True
    elif not run_wiki_checks():
        failed = True

    if ratchet_proc is not None:
        if ratchet_proc.returncode != 0:
            if ratchet_proc.stdout:
                sys.stderr.write(ratchet_proc.stdout)
                if not ratchet_proc.stdout.endswith("\n"):
                    sys.stderr.write("\n")
            failed = True
        elif ratchet_proc.stdout:
            sys.stdout.write(ratchet_proc.stdout)
            if not ratchet_proc.stdout.endswith("\n"):
                sys.stdout.write("\n")

    if generator_tasks and not batch_failed["offline"]:
        print(f"docs lint: {len(generator_tasks)} offline generator checks passed")
    if jvm_generator_tasks and not batch_failed["JVM runtime"]:
        print(f"docs lint: {len(jvm_generator_tasks)} JVM runtime generator checks passed")

    if failed:
        return 1

    print(
        "docs lint: ledgers consistent, owned deferred/not-needed decisions, "
        "Git-only completed history, Wiki-only docs layout"
    )
    return 0


GATE_STEP_DOC_SHORTHANDS: dict[str, tuple[str, str]] = {
    "vanilla-verify": ("Vanilla workspace verify", "Vanilla verify"),
    "host-doctor": ("portable host capabilities", "host doctor"),
    "scripts-lint": ("scripts python lint", "Python lint"),
    "scripts-tests": ("scripts python tests", "Python tests"),
    "test": ("cargo test --workspace", "workspace cargo test"),
    "doctest": ("cargo test --doc --workspace", "doc tests"),
}


def check_gate_docs_sync(repo_root: Path) -> list[str]:
    gate_rs = repo_root / "crates" / "bbb-xtask" / "src" / "gate.rs"
    testing_doc = repo_root / "docs" / "wiki" / "development" / "testing-and-gates.md"
    if not gate_rs.is_file() or not testing_doc.is_file():
        return []

    gate_text = gate_rs.read_text(encoding="utf-8")
    block_match = re.search(
        r"pub\s+const\s+GATE_STEPS\s*:\s*&\[\(&str,\s*&str\)\]\s*=\s*&\[(.*?)\];",
        gate_text,
        re.DOTALL,
    )
    if block_match is None:
        return ["gate-docs-sync: could not find GATE_STEPS in crates/bbb-xtask/src/gate.rs"]

    gate_steps = re.findall(r'\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)', block_match.group(1))
    if not gate_steps:
        return ["gate-docs-sync: GATE_STEPS in crates/bbb-xtask/src/gate.rs is empty"]

    doc_text = testing_doc.read_text(encoding="utf-8")
    doc_match = re.search(
        r"## Full gate\s+.*?`cargo dev gate`[^：]*?(\d+)\s*步：(.*?)，不按",
        doc_text,
        re.DOTALL,
    )
    if doc_match is None:
        return [
            "gate-docs-sync: could not find Full gate step list in "
            "docs/wiki/development/testing-and-gates.md"
        ]

    errors: list[str] = []
    declared_count = int(doc_match.group(1))
    if declared_count != len(gate_steps):
        errors.append(
            f"gate-docs-sync: testing-and-gates.md declares {declared_count} steps, "
            f"but GATE_STEPS has {len(gate_steps)}"
        )

    raw_list = " ".join(doc_match.group(2).splitlines())
    doc_steps = [part.strip() for part in raw_list.split("、") if part.strip()]
    if doc_steps and " 与 " in doc_steps[-1]:
        head, tail = doc_steps[-1].split(" 与 ", 1)
        doc_steps = [*doc_steps[:-1], head.strip(), tail.strip()]
    elif doc_steps and doc_steps[-1].startswith("与 "):
        doc_steps[-1] = doc_steps[-1][2:].strip()

    if len(doc_steps) != len(gate_steps):
        errors.append(
            f"gate-docs-sync: testing-and-gates.md lists {len(doc_steps)} steps, "
            f"but GATE_STEPS has {len(gate_steps)}"
        )
        return errors

    for index, ((step_id, step_label), doc_step) in enumerate(zip(gate_steps, doc_steps), start=1):
        doc_norm = " ".join(doc_step.replace("`", "").split())
        shorthand = GATE_STEP_DOC_SHORTHANDS.get(step_id)
        if shorthand is not None:
            expected_label, expected_doc = shorthand
            if step_label != expected_label:
                errors.append(
                    f"gate-docs-sync: step {index} ({step_id}) label {step_label!r} "
                    f"!= expected {expected_label!r}"
                )
            if expected_doc.lower() not in doc_norm.lower():
                errors.append(
                    f"gate-docs-sync: step {index} ({step_id}) doc {doc_step!r} "
                    f"missing {expected_doc!r}"
                )
        elif step_label.lower() not in doc_norm.lower():
            errors.append(
                f"gate-docs-sync: step {index} ({step_id}) doc {doc_step!r} "
                f"does not match GATE_STEPS label {step_label!r}"
            )
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    default_root = os.environ.get("BBB_DOCS_LINT_REPO_ROOT")
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(default_root) if default_root else Path(__file__).resolve().parents[1],
    )
    parser.add_argument(
        "--skip-vanilla",
        action="store_true",
        help="run repository/Wiki/ledger/ratchet checks without requiring a prepared Vanilla workspace",
    )
    parser.add_argument(
        "--all-generators",
        action="store_true",
        help="include slower offline Vanilla-source and JVM-runtime-backed generators",
    )
    args = parser.parse_args(argv)
    return run_docs_lint(
        args.repo_root.resolve(),
        skip_vanilla=args.skip_vanilla,
        all_generators=args.all_generators,
    )


if __name__ == "__main__":
    raise SystemExit(main())
