#!/usr/bin/env python3
"""Validate the repo-owned Chinese Wiki without network access."""

from __future__ import annotations

import argparse
from collections import Counter, deque
import hashlib
import json
import os
from pathlib import Path
import re
import sys


ALLOWED_STATUSES = {
    "equivalent",
    "partial",
    "missing",
    "active",
    "deferred-external",
    "not-needed",
    "unverified",
}
OPEN_STATUSES = {"partial", "missing", "active", "deferred-external", "unverified"}
ROUTED_STATUSES = OPEN_STATUSES | {"not-needed"}
LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
CODE_RE = re.compile(r"`([^`\n]+)`")
MECHANISM_RE = re.compile(r"^机制 ID：`([a-z0-9]+(?:-[a-z0-9]+)*)`\s*$", re.MULTILINE)
INVENTORY_RE = re.compile(r"^\| `([^`]+\.md)` \|", re.MULTILINE)
LEDGER_ROUTE_RE = re.compile(
    r"^Matrix routes?:\s*((?:`[a-z0-9]+(?:-[a-z0-9]+)*`(?:,?\s*)?)+)\s*$",
    re.MULTILINE,
)


class WikiLint:
    def __init__(self, repo: Path) -> None:
        self.repo = repo.resolve()
        self.issues: list[str] = []

    def fail(self, message: str) -> None:
        self.issues.append(message)

    @staticmethod
    def sha256_file(path: Path) -> str:
        with path.open("rb") as handle:
            return hashlib.file_digest(handle, "sha256").hexdigest()

    def markdown_files(self) -> list[Path]:
        return sorted(
            path
            for path in self.repo.rglob("*.md")
            if ".git" not in path.relative_to(self.repo).parts
        )

    def wiki_files(self) -> list[Path]:
        root = self.repo / "docs" / "wiki"
        return sorted(root.rglob("*.md")) if root.is_dir() else []

    @staticmethod
    def links(path: Path) -> list[str]:
        return LINK_RE.findall(path.read_text(encoding="utf-8"))

    def resolve_link(self, source: Path, raw_target: str) -> Path | None:
        target = raw_target.strip()
        if target.startswith("<") and target.endswith(">"):
            target = target[1:-1]
        if not target or target.startswith(("#", "http://", "https://", "mailto:")):
            return None
        target = target.split("#", 1)[0]
        if not target:
            return None
        return (source.parent / target).resolve()

    def check_entries(self) -> None:
        for relative in ("docs/README.md", "docs/wiki/README.md"):
            path = self.repo / relative
            if not path.is_file():
                self.fail(f"wiki entry missing: {relative}")

    def check_links(self, files: list[Path]) -> None:
        for source in files:
            for raw_target in self.links(source):
                target = self.resolve_link(source, raw_target)
                if target is None:
                    continue
                try:
                    target.relative_to(self.repo)
                except ValueError:
                    self.fail(
                        f"broken relative Markdown link escapes repo: "
                        f"{source.relative_to(self.repo)} -> {raw_target}"
                    )
                    continue
                if not target.exists():
                    self.fail(
                        f"broken relative Markdown link: "
                        f"{source.relative_to(self.repo)} -> {raw_target}"
                    )

    def check_orphans(self, files: list[Path]) -> None:
        root = self.repo / "docs" / "wiki" / "README.md"
        if not root.is_file():
            return
        wiki_set = {path.resolve() for path in files}
        reached: set[Path] = set()
        queue: deque[Path] = deque([root.resolve()])
        while queue:
            source = queue.popleft()
            if source in reached or source not in wiki_set:
                continue
            reached.add(source)
            for raw_target in self.links(source):
                target = self.resolve_link(source, raw_target)
                if target in wiki_set and target not in reached:
                    queue.append(target)
        for orphan in sorted(wiki_set - reached):
            self.fail(
                f"orphan canonical Wiki page: {orphan.relative_to(self.repo)}"
            )

    def check_mechanisms(self, files: list[Path]) -> None:
        owners: dict[str, list[Path]] = {}
        required_sections = (
            "## 机制目的和边界",
            "## Vanilla 26.1 权威实现",
            "## 决策输入与状态所有权",
            "## 判断与处理顺序",
            "## 生命周期和 fallback",
            "## BBB 实现映射",
            "## Vanilla/BBB 对照",
            "| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |",
            "## 结构性不变量",
            "## 聚焦测试（focused tests）和真实证据",
            "## 当前差异",
            "## 账本/goal 路由",
            "## 精确重开条件",
            "## 相关页面和源码索引",
        )
        for path in files:
            text = path.read_text(encoding="utf-8")
            mechanism_ids = MECHANISM_RE.findall(text)
            for mechanism_id in mechanism_ids:
                owners.setdefault(mechanism_id, []).append(path)
            if mechanism_ids:
                for section in required_sections:
                    if section not in text:
                        self.fail(
                            f"mechanism page missing required section {section!r}: "
                            f"{path.relative_to(self.repo)}"
                        )
        for mechanism_id, paths in sorted(owners.items()):
            if len(paths) != 1:
                rendered = ", ".join(str(path.relative_to(self.repo)) for path in paths)
                self.fail(f"duplicate mechanism id {mechanism_id}: {rendered}")

    def check_parity(self) -> dict[str, dict[str, str]]:
        matrix = self.repo / "docs" / "wiki" / "parity" / "vanilla-26.1-matrix.md"
        if not matrix.is_file():
            self.fail("parity matrix missing: docs/wiki/parity/vanilla-26.1-matrix.md")
            return {}
        seen: set[str] = set()
        rows: dict[str, dict[str, str]] = {}
        for line_number, line in enumerate(
            matrix.read_text(encoding="utf-8").splitlines(), start=1
        ):
            if not line.startswith("| `"):
                continue
            cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
            if len(cells) != 8:
                self.fail(f"malformed parity row at {matrix.relative_to(self.repo)}:{line_number}")
                continue
            mechanism_id = cells[0].strip("`")
            if re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", mechanism_id) is None:
                self.fail(f"illegal parity mechanism id {mechanism_id!r}")
            if mechanism_id in seen:
                self.fail(f"duplicate parity mechanism id {mechanism_id}")
            seen.add(mechanism_id)
            status_match = re.fullmatch(r"`([^`]+)`", cells[5])
            status = status_match.group(1) if status_match else ""
            if status not in ALLOWED_STATUSES:
                self.fail(
                    f"illegal parity status {status or cells[5]!r} for {mechanism_id}"
                )
            if status in ROUTED_STATUSES and not re.search(
                r"unsupported-features\.md", " ".join(cells[6:8])
            ):
                self.fail(
                    f"invalid open-state route for {mechanism_id}: "
                    "link docs/unsupported-features.md"
                )
            rows[mechanism_id] = {
                "bbb_owner": cells[3],
                "evidence": cells[6],
                "route": cells[7],
                "status": status,
            }
        return rows

    @staticmethod
    def strict_json(path: Path) -> object:
        def object_pairs(pairs):
            value = {}
            for key, item in pairs:
                if key in value:
                    raise ValueError(f"duplicate JSON key {key!r}")
                value[key] = item
            return value

        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=object_pairs)

    def check_client_mechanism_inventory(
        self, rows: dict[str, dict[str, str]]
    ) -> str:
        relative = Path("docs/vanilla-client-mechanism-inventory-26.1.json")
        path = self.repo / relative
        if not path.is_file():
            self.fail(f"client mechanism inventory missing: {relative}")
            return "client mechanism inventory missing"
        try:
            value = self.strict_json(path)
        except (OSError, ValueError, json.JSONDecodeError) as error:
            self.fail(f"invalid client mechanism inventory: {error}")
            return "client mechanism inventory invalid"
        if not isinstance(value, dict) or value.get("schema_version") != 1:
            self.fail("invalid client mechanism inventory schema")
            return "client mechanism inventory invalid"
        if value.get("vanilla_version") != "26.1":
            self.fail("client mechanism inventory has the wrong Vanilla version")

        owners = value.get("owners")
        mechanisms = value.get("mechanisms")
        source_files = value.get("source_files")
        generator_inputs = value.get("generator_inputs")
        summary = value.get("summary")
        if not isinstance(owners, list) or not isinstance(mechanisms, dict):
            self.fail("client mechanism inventory owners/mechanisms are malformed")
            return "client mechanism inventory invalid"
        if not isinstance(source_files, list) or not isinstance(generator_inputs, list):
            self.fail("client mechanism inventory source hash lists are malformed")
            return "client mechanism inventory invalid"

        owner_ids: list[str] = []
        owner_mechanisms: Counter[str] = Counter()
        owner_categories: Counter[str] = Counter()
        owner_domains: Counter[str] = Counter()
        declared_sources: set[str] = set()
        for item in source_files:
            if not isinstance(item, dict):
                self.fail("client mechanism inventory source entry is malformed")
                continue
            source_path = item.get("path")
            digest = item.get("sha256")
            if (
                not isinstance(source_path, str)
                or Path(source_path).is_absolute()
                or ".." in Path(source_path).parts
                or not isinstance(digest, str)
                or re.fullmatch(r"[0-9a-f]{64}", digest) is None
            ):
                self.fail(f"invalid client mechanism source hash entry: {item!r}")
                continue
            if source_path in declared_sources:
                self.fail(f"duplicate client mechanism source path {source_path}")
            declared_sources.add(source_path)

        for item in owners:
            if not isinstance(item, dict):
                self.fail("client mechanism inventory owner is malformed")
                continue
            owner_id = item.get("owner_id")
            mechanism_id = item.get("mechanism_id")
            source = item.get("source")
            category = item.get("category")
            domain = item.get("domain")
            scope = item.get("scope")
            vanilla_owner = item.get("vanilla_owner")
            if not isinstance(owner_id, str) or not owner_id:
                self.fail(f"client mechanism inventory owner lacks a stable id: {item!r}")
                continue
            owner_ids.append(owner_id)
            if not isinstance(mechanism_id, str) or mechanism_id not in mechanisms:
                self.fail(f"unmapped Vanilla owner {owner_id}: {mechanism_id!r}")
            else:
                owner_mechanisms[mechanism_id] += 1
            if isinstance(category, str):
                owner_categories[category] += 1
            if isinstance(domain, str):
                owner_domains[domain] += 1
            if scope not in {"external-client", "out-of-scope"}:
                self.fail(f"invalid scope for Vanilla owner {owner_id}: {scope!r}")
            if not isinstance(vanilla_owner, str) or not vanilla_owner:
                self.fail(f"Vanilla owner {owner_id} lacks a source-derived owner label")
            if (
                not isinstance(source, dict)
                or source.get("path") not in declared_sources
                or not isinstance(source.get("line"), int)
                or source.get("line", 0) < 1
            ):
                self.fail(f"invalid source locator for Vanilla owner {owner_id}")

        duplicates = sorted(owner_id for owner_id, count in Counter(owner_ids).items() if count != 1)
        if duplicates:
            self.fail("duplicate Vanilla owner ids: " + ", ".join(duplicates[:10]))
        if owner_ids != sorted(owner_ids):
            self.fail("client mechanism inventory owners are not stably sorted")

        self.check_reload_listener_projections(owners, mechanisms)

        for mechanism_id, mechanism in mechanisms.items():
            if mechanism_id not in rows:
                self.fail(f"unmapped inventory mechanism id {mechanism_id}")
            if not isinstance(mechanism, dict):
                self.fail(f"malformed inventory mechanism {mechanism_id}")
                continue
            if mechanism.get("owner_count") != owner_mechanisms[mechanism_id]:
                self.fail(f"stale owner count for inventory mechanism {mechanism_id}")
            evidence = mechanism.get("bbb_evidence")
            if not isinstance(evidence, list) or not evidence:
                self.fail(f"inventory mechanism {mechanism_id} lacks BBB evidence")
                continue
            for evidence_path in evidence:
                if (
                    not isinstance(evidence_path, str)
                    or Path(evidence_path).is_absolute()
                    or ".." in Path(evidence_path).parts
                    or not (self.repo / evidence_path.split("#", 1)[0]).exists()
                ):
                    self.fail(
                        f"stale BBB evidence path for inventory mechanism "
                        f"{mechanism_id}: {evidence_path!r}"
                    )

        expected_summary = {
            "by_category": dict(sorted(owner_categories.items())),
            "by_domain": dict(sorted(owner_domains.items())),
            "mapped_owner_count": len(owner_ids),
            "owner_count": len(owner_ids),
            "unmapped_owner_count": 0,
        }
        if summary != expected_summary:
            self.fail("client mechanism inventory summary is stale or inconsistent")

        for item in generator_inputs:
            if not isinstance(item, dict) or not isinstance(item.get("path"), str):
                self.fail("client mechanism generator input is malformed")
                continue
            input_path = item["path"]
            candidate = self.repo / input_path
            expected = item.get("sha256")
            if not candidate.is_file():
                self.fail(f"client mechanism generator input missing: {input_path}")
            elif self.sha256_file(candidate) != expected:
                self.fail(f"client mechanism inventory generator input is stale: {input_path}")

        source_root_value = os.environ.get("BBB_DOCS_LINT_VANILLA_SOURCE_ROOT")
        source_root = (
            Path(source_root_value)
            if source_root_value
            else Path.home() / "Work" / "mc-code" / "sources" / "26.1"
        )
        checked_sources = 0
        if source_root.is_dir():
            for item in source_files:
                if not isinstance(item, dict) or not isinstance(item.get("path"), str):
                    continue
                source_path = item["path"]
                candidate = source_root / source_path
                if not candidate.is_file():
                    self.fail(f"client mechanism Vanilla source missing: {source_path}")
                    continue
                checked_sources += 1
                if self.sha256_file(candidate) != item.get("sha256"):
                    self.fail(f"client mechanism inventory source is stale: {source_path}")
            source_message = f"{checked_sources} source hashes checked"
        else:
            source_message = "local Vanilla tree absent; source hashes skipped"

        routed_ids: set[str] = set()
        ledger = self.repo / "docs" / "unsupported-features.md"
        if ledger.is_file():
            ledger_text = ledger.read_text(encoding="utf-8")
            for match in LEDGER_ROUTE_RE.finditer(ledger_text):
                routed_ids.update(CODE_RE.findall(match.group(1)))
            for mechanism_id in sorted(routed_ids - set(rows)):
                self.fail(f"ledger route references unknown mechanism id {mechanism_id}")
            for mechanism_id, row in rows.items():
                if row["status"] in ROUTED_STATUSES and mechanism_id not in routed_ids:
                    self.fail(f"parity row {mechanism_id} has no bilateral ledger route")

            top = ""
            section = ""
            section_has_item = False
            section_has_route = False

            def flush_section() -> None:
                if top in {"Todo", "Not needed"} and section_has_item and not section_has_route:
                    self.fail(f"ledger section lacks Matrix routes: {top} / {section}")

            for line in ledger_text.splitlines():
                if line.startswith("# "):
                    flush_section()
                    top = line[2:].strip()
                    section = ""
                    section_has_item = False
                    section_has_route = False
                elif line.startswith("## "):
                    flush_section()
                    section = line[3:].strip()
                    section_has_item = False
                    section_has_route = False
                elif line.startswith("- ") and top in {"Todo", "Not needed"}:
                    section_has_item = True
                elif LEDGER_ROUTE_RE.fullmatch(line):
                    section_has_route = True
            flush_section()

        for mechanism_id, row in rows.items():
            if row["status"] != "equivalent":
                continue
            mechanism = mechanisms.get(mechanism_id)
            inventory_evidence = (
                isinstance(mechanism, dict)
                and isinstance(mechanism.get("bbb_evidence"), list)
                and bool(mechanism["bbb_evidence"])
            )
            locatable_cell = any(
                (self.repo / token.split("#", 1)[0]).exists()
                for token in CODE_RE.findall(row["evidence"])
                if token.startswith(("docs/", "scripts/", "crates/"))
            ) or bool(LINK_RE.search(row["evidence"]))
            if not inventory_evidence and not locatable_cell:
                self.fail(f"equivalent parity row {mechanism_id} lacks locatable evidence")

        return (
            f"client inventory {len(owner_ids)} owners/{len(mechanisms)} mechanisms, "
            f"unmapped 0, {source_message}"
        )

    def check_reload_listener_projections(
        self, owners: list[object], mechanisms: dict[str, object]
    ) -> None:
        mechanism_id = "resources-reload-listeners"
        if mechanism_id not in mechanisms:
            return
        relative = Path("docs/wiki/mechanisms/resources/offline-asset-trust.md")
        path = self.repo / relative
        if not path.is_file():
            self.fail(f"reload-listener projection table missing: {relative}")
            return
        text = path.read_text(encoding="utf-8")
        start_marker = "<!-- reload-listener-projections:start -->"
        end_marker = "<!-- reload-listener-projections:end -->"
        if text.count(start_marker) != 1 or text.count(end_marker) != 1:
            self.fail("reload-listener projection table markers are missing or duplicated")
            return
        table = text.split(start_marker, 1)[1].split(end_marker, 1)[0]
        projection_rows: list[tuple[int, str, str]] = []
        for line in table.splitlines():
            if re.fullmatch(r"\|\s*\d+\s*\|.*", line) is None:
                continue
            cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
            if len(cells) != 7:
                self.fail(f"malformed reload-listener projection row: {line}")
                continue
            owner_match = re.fullmatch(r"`([^`]+)`", cells[1])
            status_match = re.fullmatch(r"`([^`]+)`", cells[5])
            if owner_match is None or status_match is None:
                self.fail(f"malformed reload-listener owner/status cell: {line}")
                continue
            status = status_match.group(1)
            if status not in {"existing", "active", "not-needed"}:
                self.fail(f"illegal reload-listener projection status {status!r}")
            projection_rows.append((int(cells[0]), owner_match.group(1), status))

        def source_order(item: dict[str, object]) -> tuple[object, object, object]:
            source = item.get("source")
            if not isinstance(source, dict):
                source = {}
            return (
                source.get("path", ""),
                source.get("line", 0),
                item.get("owner_id", ""),
            )

        expected = sorted(
            (
                item
                for item in owners
                if isinstance(item, dict)
                and item.get("mechanism_id") == mechanism_id
            ),
            key=source_order,
        )
        expected_rows = [
            (index, item.get("owner_id"))
            for index, item in enumerate(expected, start=1)
        ]
        actual_rows = [
            (index, owner_id) for index, owner_id, _status in projection_rows
        ]
        if actual_rows != expected_rows:
            self.fail(
                "reload-listener projection table is stale: "
                f"expected {len(expected_rows)} source-ordered owners, "
                f"found {len(actual_rows)}"
            )

    def check_inventory(self, all_markdown: list[Path]) -> None:
        inventory = (
            self.repo / "docs" / "wiki" / "development" / "document-inventory.md"
        )
        if not inventory.is_file():
            self.fail("document inventory missing")
            return
        declared = Counter(INVENTORY_RE.findall(inventory.read_text(encoding="utf-8")))
        actual = {str(path.relative_to(self.repo)) for path in all_markdown}
        duplicates = sorted(path for path, count in declared.items() if count != 1)
        if duplicates:
            self.fail("document inventory duplicate rows: " + ", ".join(duplicates))
        missing = sorted(actual - set(declared))
        extra = sorted(set(declared) - actual)
        if missing:
            self.fail("document inventory missing paths: " + ", ".join(missing))
        if extra:
            self.fail("document inventory stale paths: " + ", ".join(extra))

    def check_repo_paths(self, files: list[Path]) -> None:
        prefixes = ("README.md", "AGENTS.md", "goal.md", "docs/", "scripts/", "crates/")
        for source in files:
            for token in CODE_RE.findall(source.read_text(encoding="utf-8")):
                if not token.startswith(prefixes):
                    continue
                if any(char in token for char in (" ", "*", "<", ">", "|")):
                    continue
                candidate = token.split("#", 1)[0].split("::", 1)[0].rstrip(".,;:")
                if not candidate:
                    continue
                if not (self.repo / candidate).exists():
                    self.fail(
                        f"declared repo-local path missing: "
                        f"{source.relative_to(self.repo)} -> {candidate}"
                    )

    def check_chinese_and_stale_text(self, files: list[Path]) -> None:
        banned = ("RESOLVED (this slice)", "snapshot statistics", "TODO")
        for path in files:
            text = path.read_text(encoding="utf-8")
            relative = path.relative_to(self.repo)
            if len(re.findall(r"[\u3400-\u9fff]", text)) < 30:
                self.fail(f"Chinese Wiki page is placeholder/mostly non-Chinese: {relative}")
            if len([line for line in text.splitlines() if line.strip()]) < 5:
                self.fail(f"Chinese Wiki page has only a title/placeholder: {relative}")
            for phrase in banned:
                if phrase in text:
                    self.fail(f"stale completion prose {phrase!r} in {relative}")

    def check_document_layout(self, files: list[Path]) -> None:
        allowed_top_level = {
            Path("docs/README.md"),
            Path("docs/goal-archive.md"),
            Path("docs/unsupported-features.md"),
        }
        for path in files:
            relative = path.relative_to(self.repo)
            if relative in allowed_top_level:
                continue
            if relative.parts[:2] == ("docs", "wiki"):
                continue
            if relative.parts and relative.parts[0] == "docs":
                self.fail(
                    "historical Markdown outside the Wiki/flow allowlist: "
                    f"{relative}"
                )

    def check_vanilla_sources(self) -> str:
        source_root = Path.home() / "Work" / "mc-code" / "sources" / "26.1"
        if not source_root.is_dir():
            return "local Vanilla 26.1 sources absent; source-location checks skipped"
        required = [
            "net/minecraft/client/main/Main.java",
            "net/minecraft/client/Minecraft.java",
            "net/minecraft/client/multiplayer/ClientPacketListener.java",
            "net/minecraft/client/renderer/block/BlockModelLighter.java",
            "net/minecraft/client/renderer/block/FluidRenderer.java",
            "net/minecraft/client/gui/font/FontSet.java",
            "net/minecraft/client/gui/font/providers/UnihexProvider.java",
        ]
        missing = [relative for relative in required if not (source_root / relative).is_file()]
        if missing:
            self.fail("local Vanilla source locations missing: " + ", ".join(missing))
        return f"local Vanilla source locations checked: {len(required)}"

    def run(self) -> int:
        all_markdown = self.markdown_files()
        wiki = self.wiki_files()
        self.check_entries()
        self.check_links(all_markdown)
        self.check_orphans(wiki)
        self.check_mechanisms(wiki)
        parity_rows = self.check_parity()
        client_inventory_message = self.check_client_mechanism_inventory(parity_rows)
        self.check_inventory(all_markdown)
        self.check_repo_paths(wiki)
        self.check_chinese_and_stale_text(wiki)
        self.check_document_layout(all_markdown)
        source_message = self.check_vanilla_sources()
        if self.issues:
            for issue in self.issues:
                print(f"wiki lint: {issue}", file=sys.stderr)
            return 1
        print(
            f"wiki lint: {len(wiki)} reachable Chinese pages, "
            f"{len(all_markdown)} inventoried Markdown files, links/ids/status/routes valid; "
            f"{source_message}; {client_inventory_message}"
        )
        return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    args = parser.parse_args()
    return WikiLint(args.repo_root).run()


if __name__ == "__main__":
    raise SystemExit(main())
