"""Wiki lint helper module imported by scripts/docs_lint.py."""

from __future__ import annotations

import os
import re
import sys
import unicodedata
from collections import Counter, deque
from pathlib import Path
from urllib.parse import unquote

from bbb_tools.vanilla.errors import VanillaError
from bbb_tools.vanilla.workspace import VanillaWorkspace

LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
CODE_RE = re.compile(r"`([^`\n]+)`")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$")
HTML_ANCHOR_RE = re.compile(r'<a\s+[^>]*(?:id|name)=["\']([^"\']+)["\'][^>]*>', re.IGNORECASE)
INLINE_LINK_RE = re.compile(r"\[([^\]]+)\]\([^)]+\)")
MECHANISM_RE = re.compile(r"^机制 ID：`([a-z0-9]+(?:-[a-z0-9]+)*)`\s*$", re.MULTILINE)
EXTENSION_RE = re.compile(
    r"^扩展 ID：`(client-extension/[a-z0-9]+(?:-[a-z0-9]+)*)`\s*$", re.MULTILINE
)
INVENTORY_RE = re.compile(r"^\| `([^`]+\.md)` \|", re.MULTILINE)
MECHANISM_TABLE_HEADER = "| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |"
ALLOWED_MECHANISM_STATUSES = frozenset(
    {
        "aligned",
        "equivalent",
        "partial",
        "missing",
        "active",
        "deferred-external",
        "not-needed",
        "unverified",
    }
)


def _iter_non_fenced_lines(text: str):
    """Yield (1-based lineno, line) outside fenced Markdown code blocks."""
    fence_char = ""
    fence_len = 0
    for lineno, line in enumerate(text.splitlines(), 1):
        stripped = line.lstrip()
        if not fence_char:
            if stripped.startswith(("```", "~~~")):
                ch = stripped[0]
                count = len(stripped) - len(stripped.lstrip(ch))
                if count >= 3:
                    fence_char = ch
                    fence_len = count
                    continue
            yield lineno, line
        else:
            if stripped.startswith(fence_char * fence_len) and not stripped.lstrip(fence_char).strip():
                fence_char = ""
                fence_len = 0


def _strip_inline_markdown(text: str) -> str:
    plain = INLINE_LINK_RE.sub(r"\1", text)
    return plain.replace("`", "").replace("*", "").replace("~", "")


def github_heading_slug(heading: str) -> str:
    """Compute a GitHub Flavored Markdown heading slug (preserving CJK and ASCII word chars)."""
    plain = _strip_inline_markdown(heading).strip().lower()
    out: list[str] = []
    for ch in plain:
        if ch == " ":
            out.append("-")
        elif ch in ("-", "_"):
            out.append(ch)
        elif unicodedata.category(ch)[0] in ("L", "N"):
            out.append(ch)
    return "".join(out)


def markdown_anchors(text: str) -> set[str]:
    """Collect all valid GitHub heading slugs and explicit HTML anchors in a Markdown document."""
    anchors: set[str] = set()
    slug_counts: Counter[str] = Counter()
    for _lineno, line in _iter_non_fenced_lines(text):
        for html_id in HTML_ANCHOR_RE.findall(line):
            cleaned = html_id.strip().lower()
            if cleaned:
                anchors.add(cleaned)
        match = HEADING_RE.match(line)
        if not match:
            continue
        base = github_heading_slug(match.group(2))
        if not base:
            continue
        seen = slug_counts[base]
        slug_counts[base] += 1
        anchors.add(base if seen == 0 else f"{base}-{seen}")
    return anchors


IGNORED_MARKDOWN_DIRS = frozenset({".git", ".venv", ".local", "target"})


class WikiLint:
    def __init__(self, repo: Path, *, check_vanilla: bool = True) -> None:
        self.repo = repo.resolve()
        self.check_vanilla = check_vanilla
        self.issues: list[str] = []

    def fail(self, message: str) -> None:
        self.issues.append(message)

    def markdown_files(self) -> list[Path]:
        found: list[Path] = []
        for current, dirnames, filenames in os.walk(self.repo):
            dirnames[:] = [d for d in dirnames if d not in IGNORED_MARKDOWN_DIRS]
            current_path = Path(current)
            for name in filenames:
                if name.endswith(".md"):
                    found.append(current_path / name)
        return sorted(found)

    def wiki_files(self) -> list[Path]:
        root = self.repo / "docs" / "wiki"
        return sorted(root.rglob("*.md")) if root.is_dir() else []

    @staticmethod
    def links(path: Path) -> list[str]:
        found: list[str] = []
        for _lineno, line in _iter_non_fenced_lines(path.read_text(encoding="utf-8")):
            found.extend(LINK_RE.findall(line))
        return found

    def resolve_link_with_fragment(
        self, source: Path, raw_target: str
    ) -> tuple[Path | None, str | None]:
        target = raw_target.strip()
        if target.startswith("<") and target.endswith(">"):
            target = target[1:-1]
        if not target or target.startswith(("http://", "https://", "mailto:", "javascript:")):
            return None, None
        if target.startswith("#"):
            return source.resolve(), target[1:]
        if "#" in target:
            path_part, fragment = target.split("#", 1)
        else:
            path_part, fragment = target, None
        if not path_part:
            return source.resolve(), fragment
        return (source.parent / path_part).resolve(), fragment

    def resolve_link(self, source: Path, raw_target: str) -> Path | None:
        target = raw_target.strip()
        if target.startswith("<") and target.endswith(">"):
            target = target[1:-1]
        if not target or target.startswith(("#", "http://", "https://", "mailto:", "javascript:")):
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
        anchors_cache: dict[Path, set[str]] = {}

        def anchors_for(path: Path) -> set[str]:
            cached = anchors_cache.get(path)
            if cached is None:
                cached = markdown_anchors(path.read_text(encoding="utf-8"))
                anchors_cache[path] = cached
            return cached

        for source in files:
            for raw_target in self.links(source):
                target, fragment = self.resolve_link_with_fragment(source, raw_target)
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
                    continue
                if fragment is not None and target.is_file() and target.suffix.lower() == ".md":
                    wanted = unquote(fragment).strip().lower()
                    if not wanted or wanted not in anchors_for(target):
                        self.fail(
                            f"broken Markdown heading anchor: "
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
        extension_owners: dict[str, list[Path]] = {}
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
            "## 账本归属",
            "## 精确重开条件",
            "## 相关页面和源码索引",
        )
        extension_required_sections = (
            "## 机制目的和边界",
            "## BBB 行为合同",
            "## 决策输入与状态所有权",
            "## 判断与处理顺序",
            "## 生命周期和 fallback",
            "## BBB 实现映射",
            "## 启用方式与服务端能力",
            "## 结构性不变量",
            "## 聚焦测试（focused tests）和真实证据",
            "## 当前差异",
            "## 账本归属",
            "## 精确重开条件",
            "## 相关页面和源码索引",
        )
        for path in files:
            relative = path.relative_to(self.repo)
            in_mechanisms_dir = relative.parts[:3] == ("docs", "wiki", "mechanisms")
            text = path.read_text(encoding="utf-8")
            mechanism_ids = MECHANISM_RE.findall(text)
            extension_ids = EXTENSION_RE.findall(text)
            for mechanism_id in mechanism_ids:
                owners.setdefault(mechanism_id, []).append(path)
                if in_mechanisms_dir and path.name.startswith("server-") and not mechanism_id.startswith("server-"):
                    self.fail(
                        f"server mechanism page must use a 'server-' prefixed 机制 ID "
                        f"(found {mechanism_id!r}): {relative}"
                    )
            for extension_id in extension_ids:
                extension_owners.setdefault(extension_id, []).append(path)
            if mechanism_ids and extension_ids:
                self.fail(
                    f"mechanism page cannot declare both 机制 ID and 扩展 ID: {relative}"
                )
            if in_mechanisms_dir and not mechanism_ids and not extension_ids:
                self.fail(
                    f"mechanism page under docs/wiki/mechanisms/ missing 机制 ID or 扩展 ID: {relative}"
                )
            if mechanism_ids:
                for section in required_sections:
                    if section not in text:
                        self.fail(
                            f"mechanism page missing required section {section!r}: "
                            f"{relative}"
                        )
                self._check_mechanism_table_statuses(relative, text)
            elif extension_ids:
                for section in extension_required_sections:
                    if section not in text:
                        self.fail(
                            f"extension mechanism page missing required section {section!r}: "
                            f"{relative}"
                        )
        for mechanism_id, paths in sorted(owners.items()):
            if len(paths) != 1:
                rendered = ", ".join(str(path.relative_to(self.repo)) for path in paths)
                self.fail(f"duplicate mechanism id {mechanism_id}: {rendered}")
        for extension_id, paths in sorted(extension_owners.items()):
            if len(paths) != 1:
                rendered = ", ".join(str(path.relative_to(self.repo)) for path in paths)
                self.fail(f"duplicate extension id {extension_id}: {rendered}")

    def _check_mechanism_table_statuses(self, relative: Path, text: str) -> None:
        in_table = False
        allowed_display = ", ".join(f"`{s}`" for s in sorted(ALLOWED_MECHANISM_STATUSES))
        for lineno, line in _iter_non_fenced_lines(text):
            stripped = line.strip()
            if stripped == MECHANISM_TABLE_HEADER:
                in_table = True
                continue
            if not in_table:
                continue
            if not stripped.startswith("|"):
                in_table = False
                continue
            cells = [cell.strip() for cell in stripped.strip("|").split("|")]
            if all(cell and set(cell) <= {"-", ":"} for cell in cells):
                continue
            if len(cells) < 4:
                self.fail(
                    f"malformed Vanilla/BBB table row in {relative}:{lineno}"
                )
                continue
            raw_status = cells[3]
            if (
                not (raw_status.startswith("`") and raw_status.endswith("`"))
                or raw_status[1:-1] not in ALLOWED_MECHANISM_STATUSES
            ):
                self.fail(
                    f"invalid Vanilla/BBB table status {raw_status!r} in {relative}:{lineno} "
                    f"(expected one of: {allowed_display})"
                )

    def check_inventory(self, all_markdown: list[Path]) -> None:
        inventory = (
            self.repo / "docs" / "wiki" / "development" / "document-inventory.md"
        )
        if not inventory.is_file():
            self.fail("document inventory missing")
            return
        declared = Counter(INVENTORY_RE.findall(inventory.read_text(encoding="utf-8")))
        actual = {path.relative_to(self.repo).as_posix() for path in all_markdown}
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
            Path("docs/unsupported-client-features.md"),
            Path("docs/unsupported-server-features.md"),
            Path("docs/client-extensions.md"),
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
        try:
            source_root = VanillaWorkspace.resolve("26.1").require_prepared().sources
        except VanillaError as error:
            self.fail(f"vanilla {error.kind}: {error}")
            return "local Vanilla source locations unavailable"
        required = [
            "net/minecraft/client/main/Main.java",
            "net/minecraft/client/Minecraft.java",
            "net/minecraft/client/multiplayer/ClientPacketListener.java",
            "net/minecraft/client/renderer/block/BlockModelLighter.java",
            "net/minecraft/client/renderer/block/FluidRenderer.java",
            "net/minecraft/client/gui/font/FontSet.java",
            "net/minecraft/client/gui/font/providers/UnihexProvider.java",
            "net/minecraft/server/Main.java",
            "net/minecraft/server/MinecraftServer.java",
            "net/minecraft/server/dedicated/DedicatedServer.java",
            "net/minecraft/server/level/ServerLevel.java",
            "net/minecraft/server/network/ServerConnectionListener.java",
            "net/minecraft/server/network/ServerLoginPacketListenerImpl.java",
            "net/minecraft/server/network/ServerConfigurationPacketListenerImpl.java",
            "net/minecraft/server/network/ServerGamePacketListenerImpl.java",
            "net/minecraft/server/jsonrpc/IncomingRpcMethods.java",
            "net/minecraft/server/jsonrpc/internalapi/MinecraftApi.java",
            "net/minecraft/server/jsonrpc/internalapi/MinecraftExecutorServiceImpl.java",
        ]
        missing = [relative for relative in required if not (source_root / relative).is_file()]
        if missing:
            self.fail("local Vanilla source locations missing: " + ", ".join(missing))
        return f"local Vanilla source locations checked: {len(required)}"

    def run(self) -> int:
        all_markdown = self.markdown_files()
        wiki = self.wiki_files()
        if (self.repo / "docs/client-extensions.md").exists():
            from client_extensions import load_extensions

            try:
                load_extensions(self.repo)
            except ValueError as error:
                self.fail(str(error))
        self.check_entries()
        self.check_links(all_markdown)
        self.check_orphans(wiki)
        self.check_mechanisms(wiki)
        self.check_inventory(all_markdown)
        self.check_repo_paths(wiki)
        self.check_chinese_and_stale_text(wiki)
        self.check_document_layout(all_markdown)
        source_message = (
            self.check_vanilla_sources()
            if self.check_vanilla
            else "local Vanilla source check skipped (--skip-vanilla)"
        )
        if self.issues:
            for issue in self.issues:
                print(f"wiki lint: {issue}", file=sys.stderr)
            return 1
        print(
            f"wiki lint: {len(wiki)} reachable Chinese pages, "
            f"{len(all_markdown)} inventoried Markdown files, links/ids/status valid; "
            f"{source_message}"
        )
        return 0
