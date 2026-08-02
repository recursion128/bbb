#!/usr/bin/env python3
"""Validate the repo-owned Chinese Wiki without network access."""

from __future__ import annotations

import argparse
from collections import Counter, deque
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
LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
CODE_RE = re.compile(r"`([^`\n]+)`")
MECHANISM_RE = re.compile(r"^机制 ID：`([a-z0-9]+(?:-[a-z0-9]+)*)`\s*$", re.MULTILINE)
INVENTORY_RE = re.compile(r"^\| `([^`]+\.md)` \|", re.MULTILINE)


class WikiLint:
    def __init__(self, repo: Path) -> None:
        self.repo = repo.resolve()
        self.issues: list[str] = []

    def fail(self, message: str) -> None:
        self.issues.append(message)

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

    def check_parity(self) -> None:
        matrix = self.repo / "docs" / "wiki" / "parity" / "vanilla-26.1-matrix.md"
        if not matrix.is_file():
            self.fail("parity matrix missing: docs/wiki/parity/vanilla-26.1-matrix.md")
            return
        seen: set[str] = set()
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
            if mechanism_id in seen:
                self.fail(f"duplicate parity mechanism id {mechanism_id}")
            seen.add(mechanism_id)
            status_match = re.fullmatch(r"`([^`]+)`", cells[5])
            status = status_match.group(1) if status_match else ""
            if status not in ALLOWED_STATUSES:
                self.fail(
                    f"illegal parity status {status or cells[5]!r} for {mechanism_id}"
                )
            if status in OPEN_STATUSES and not re.search(
                r"unsupported-features\.md|goal\.md", " ".join(cells[6:8])
            ):
                self.fail(
                    f"invalid open-state route for {mechanism_id}: "
                    "link goal.md or docs/unsupported-features.md"
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

    def check_canonical_scopes(self) -> None:
        required = [
            "docs/cargo-build-performance.md",
            "docs/code-organization-style.md",
            "docs/native-client-architecture.md",
        ]
        required.extend(
            str(path.relative_to(self.repo))
            for path in (self.repo / "docs").glob("*-design.md")
            if path.name != "wiki-engineering-design.md"
        )
        for relative in required:
            path = self.repo / relative
            if path.is_file() and "Canonical scope" not in path.read_text(encoding="utf-8"):
                self.fail(f"legacy current doc has no canonical-scope route: {relative}")

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
        self.check_parity()
        self.check_inventory(all_markdown)
        self.check_repo_paths(wiki)
        self.check_chinese_and_stale_text(wiki)
        self.check_canonical_scopes()
        source_message = self.check_vanilla_sources()
        if self.issues:
            for issue in self.issues:
                print(f"wiki lint: {issue}", file=sys.stderr)
            return 1
        print(
            f"wiki lint: {len(wiki)} reachable Chinese pages, "
            f"{len(all_markdown)} inventoried Markdown files, links/ids/status/routes valid; "
            f"{source_message}"
        )
        return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    args = parser.parse_args()
    return WikiLint(args.repo_root).run()


if __name__ == "__main__":
    raise SystemExit(main())
