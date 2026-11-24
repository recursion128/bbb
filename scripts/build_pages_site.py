"""Build and verify the static GitHub Pages documentation site for bbb."""

from __future__ import annotations

import argparse
import html
import os
import posixpath
import re
import shutil
import unicodedata
from collections import Counter
from pathlib import Path
from urllib.parse import unquote

REPO_ROOT = Path(__file__).resolve().parents[1]
REPO_GITHUB_URL = "https://github.com/recursion128/bbb"
IGNORED_DIRS = frozenset({".git", ".venv", ".local", "target"})

HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$")
UL_RE = re.compile(r"^(\s*)[-*+]\s+(.*)$")
OL_RE = re.compile(r"^(\s*)\d+\.\s+(.*)$")
TABLE_SEP_RE = re.compile(r"^\s*\|?(?:\s*:?-{3,}:?\s*\|)+\s*:?-{3,}:?\s*\|?\s*$")
HTML_ANCHOR_LINE_RE = re.compile(
    r'^<a\s+(?:id|name)=["\']([^"\']+)["\']\s*>\s*</a>\s*$', re.IGNORECASE
)
INLINE_LINK_RE = re.compile(r"\[([^\]]+)\]\([^)]+\)")
AUTOLINK_RE = re.compile(r"<(https?://[^>\s]+)>")
BOLD_RE = re.compile(r"\*\*([^\n]+?)\*\*")
ITALIC_RE = re.compile(r"(?<!\*)\*([^*\n]+)\*(?!\*)")
STRIKE_RE = re.compile(r"~~([^~\n]+)~~")
FENCE_RE = re.compile(r"^(\s*)(`{3,}|~{3,})(.*)$")

STYLE_CSS = """
:root {
  --bg: #0d1117;
  --panel: #161b22;
  --border: #30363d;
  --text: #e6edf3;
  --muted: #8b949e;
  --accent: #58a6ff;
  --code-bg: #1f242c;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC",
    "Microsoft YaHei", "Noto Sans SC", sans-serif;
  line-height: 1.7;
  display: flex;
  min-height: 100vh;
}
nav.sidebar {
  width: 300px;
  flex-shrink: 0;
  background: var(--panel);
  border-right: 1px solid var(--border);
  padding: 20px 16px 32px;
  overflow-y: auto;
  position: sticky;
  top: 0;
  height: 100vh;
}
nav.sidebar .brand {
  display: block;
  margin: 0 0 12px;
  padding: 4px 8px;
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: var(--text);
  text-decoration: none;
}
nav.sidebar .brand:hover {
  color: var(--accent);
  text-decoration: none;
}
nav.sidebar .ext-links {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 0 0 14px;
  padding: 0 4px;
}
nav.sidebar .ext-links a {
  display: inline-block;
  padding: 4px 8px;
  font-size: 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: #21262d;
  color: var(--text);
}
nav.sidebar .ext-links a:hover {
  border-color: var(--accent);
  color: var(--accent);
  text-decoration: none;
}
nav.sidebar .group {
  margin: 16px 0 6px;
  padding: 0 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--muted);
}
nav.sidebar a.nav-item {
  display: block;
  padding: 4px 8px;
  border-radius: 6px;
  color: var(--text);
  text-decoration: none;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
nav.sidebar a.nav-item:hover, nav.sidebar a.nav-item.active {
  background: #21262d;
  color: var(--accent);
  text-decoration: none;
}
nav.sidebar a.nav-item.active {
  font-weight: 600;
}
main.content {
  flex: 1;
  min-width: 0;
  max-width: 980px;
  padding: 28px 40px 64px;
}
.page-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  font-size: 12px;
  color: var(--muted);
  margin-bottom: 16px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border);
}
.page-meta code {
  font-size: 12px;
}
h1, h2, h3, h4 {
  margin-top: 1.5em;
  margin-bottom: 0.6em;
  border-bottom: 1px solid var(--border);
  padding-bottom: 0.25em;
  scroll-margin-top: 16px;
}
main.content > h1:first-of-type { margin-top: 0.2em; }
h3, h4 { border-bottom: 0; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
code {
  background: var(--code-bg);
  padding: 0.15em 0.4em;
  border-radius: 4px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.88em;
}
pre {
  background: var(--code-bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 14px 16px;
  overflow-x: auto;
}
pre code { background: transparent; padding: 0; font-size: 0.88em; }
pre.mermaid {
  background: var(--panel);
  text-align: center;
  padding: 16px;
}
.table-wrap {
  width: 100%;
  overflow-x: auto;
  margin: 16px 0;
}
table {
  border-collapse: collapse;
  width: 100%;
  font-size: 14px;
}
th, td {
  border: 1px solid var(--border);
  padding: 8px 10px;
  text-align: left;
  vertical-align: top;
}
th { background: var(--panel); white-space: nowrap; }
ul, ol { padding-left: 1.6em; }
li { margin: 0.25em 0; }
li > p { margin: 0.35em 0; }
blockquote {
  margin: 14px 0;
  padding: 8px 14px;
  border-left: 3px solid var(--accent);
  color: var(--muted);
  background: rgba(88, 166, 255, 0.06);
}
hr {
  border: 0;
  border-top: 1px solid var(--border);
  margin: 24px 0;
}
@media (max-width: 900px) {
  body { flex-direction: column; }
  nav.sidebar {
    width: 100%;
    height: auto;
    position: static;
    border-right: 0;
    border-bottom: 1px solid var(--border);
    max-height: 45vh;
  }
  main.content { padding: 20px; }
}
""".strip()

MERMAID_SCRIPT = """  <script type="module">
    import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
    mermaid.initialize({ startOnLoad: true, theme: "dark", securityLevel: "strict" });
  </script>
"""

SIDEBAR_SCROLL_SCRIPT = """  <script>
    const active = document.querySelector("nav.sidebar a.active");
    if (active) { active.scrollIntoView({ block: "nearest" }); }
  </script>
"""


def _is_cjk_char(ch: str) -> bool:
    code = ord(ch)
    return (
        0x3000 <= code <= 0x303F
        or 0x3400 <= code <= 0x4DBF
        or 0x4E00 <= code <= 0x9FFF
        or 0xFF00 <= code <= 0xFFEF
    )


def join_wrapped_lines(lines: list[str]) -> str:
    """Join soft-wrapped Markdown lines without injecting stray spaces between CJK chars."""
    result = ""
    for raw in lines:
        part = raw.strip()
        if not part:
            continue
        if not result:
            result = part
            continue
        if _is_cjk_char(result[-1]) or _is_cjk_char(part[0]):
            result += part
        else:
            result += " " + part
    return result


def heading_slug(text: str) -> str:
    """Compute GitHub Flavored Markdown heading slug matching docs_wiki_lint.py."""
    plain = INLINE_LINK_RE.sub(r"\1", text).replace("`", "").replace("*", "").replace("~", "")
    out: list[str] = []
    for ch in plain.strip().lower():
        if ch == " ":
            out.append("-")
        elif ch in ("-", "_"):
            out.append(ch)
        elif unicodedata.category(ch)[0] in ("L", "N"):
            out.append(ch)
    return "".join(out) or "section"


def output_rel_for(source_rel: str) -> str:
    if source_rel == "README.md":
        return "index.html"
    return source_rel[:-3] + ".html"


def rewrite_link(
    raw_target: str,
    current_rel: str,
    known_pages: set[str],
    repo_root: Path = REPO_ROOT,
) -> str:
    target = raw_target.strip()
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1].strip()
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return target
    path_part, sep, frag = target.partition("#")
    if not path_part:
        return target
    current_dir = posixpath.dirname(current_rel)
    resolved = posixpath.normpath(posixpath.join(current_dir, path_part))
    if resolved in known_pages:
        target_html = output_rel_for(resolved)
        current_out_dir = posixpath.dirname(output_rel_for(current_rel))
        rel = posixpath.relpath(target_html, current_out_dir or ".")
        return f"{rel}{sep}{frag}"
    candidate = repo_root / resolved
    if candidate.is_dir():
        return f"{REPO_GITHUB_URL}/tree/main/{resolved}"
    return f"{REPO_GITHUB_URL}/blob/main/{resolved}{sep}{frag}"


def _extract_code_spans(text: str, tokens: list[str]) -> str:
    """Replace inline Markdown code spans (single or multi-backtick) with placeholders."""
    out: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        if text[i] == "`":
            ticks = 1
            while i + ticks < n and text[i + ticks] == "`":
                ticks += 1
            fence = "`" * ticks
            close = text.find(fence, i + ticks)
            if close != -1 and "\n" not in text[i + ticks : close]:
                inner = text[i + ticks : close]
                if len(inner) >= 2 and inner.startswith(" ") and inner.endswith(" ") and inner.strip():
                    inner = inner[1:-1]
                idx = len(tokens)
                tokens.append(f"<code>{html.escape(inner)}</code>")
                out.append(f"\x00CODE{idx}\x00")
                i = close + ticks
                continue
            out.append(fence)
            i += ticks
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


def _replace_markdown_links(
    text: str,
    current_rel: str,
    known_pages: set[str],
    repo_root: Path,
) -> str:
    """Replace [label](href) links while supporting balanced parentheses in href."""
    out: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        if text[i] == "[" and (i == 0 or text[i - 1] != "!"):
            close_bracket = text.find("]", i + 1)
            if close_bracket != -1 and close_bracket + 1 < n and text[close_bracket + 1] == "(":
                depth = 1
                j = close_bracket + 2
                while j < n and depth > 0:
                    if text[j] == "(":
                        depth += 1
                    elif text[j] == ")":
                        depth -= 1
                        if depth == 0:
                            break
                    j += 1
                if depth == 0 and j < n:
                    label = text[i + 1 : close_bracket]
                    href = html.unescape(text[close_bracket + 2 : j])
                    rewritten = rewrite_link(href, current_rel, known_pages, repo_root)
                    out.append(f'<a href="{html.escape(rewritten, quote=True)}">{label}</a>')
                    i = j + 1
                    continue
        out.append(text[i])
        i += 1
    return "".join(out)


def render_inline(
    text: str,
    current_rel: str,
    known_pages: set[str],
    *,
    repo_root: Path = REPO_ROOT,
    autolink_known_code_paths: bool = False,
) -> str:
    code_tokens: list[str] = []
    protected = _extract_code_spans(text, code_tokens)

    link_tokens: list[str] = []

    def stash_autolink(match: re.Match[str]) -> str:
        url = match.group(1)
        idx = len(link_tokens)
        escaped_url = html.escape(url, quote=True)
        link_tokens.append(f'<a href="{escaped_url}">{html.escape(url)}</a>')
        return f"\x00AUTOLINK{idx}\x00"

    protected = AUTOLINK_RE.sub(stash_autolink, protected)
    escaped = html.escape(protected)
    escaped = _replace_markdown_links(escaped, current_rel, known_pages, repo_root)
    escaped = BOLD_RE.sub(r"<strong>\1</strong>", escaped)
    escaped = STRIKE_RE.sub(r"<del>\1</del>", escaped)
    escaped = ITALIC_RE.sub(r"<em>\1</em>", escaped)

    for idx, snippet in enumerate(link_tokens):
        escaped = escaped.replace(f"\x00AUTOLINK{idx}\x00", snippet)
    for idx, snippet in enumerate(code_tokens):
        if autolink_known_code_paths:
            raw_code = html.unescape(snippet[len("<code>") : -len("</code>")])
            if raw_code in known_pages:
                target_html = output_rel_for(raw_code)
                current_out_dir = posixpath.dirname(output_rel_for(current_rel))
                rel = posixpath.relpath(target_html, current_out_dir or ".")
                snippet = f'<a href="{html.escape(rel, quote=True)}">{snippet}</a>'
        escaped = escaped.replace(f"\x00CODE{idx}\x00", snippet)
    return escaped


def split_table_row(line: str) -> list[str]:
    """Split a GFM table row while preserving pipes inside backtick code spans and \\| escapes."""
    stripped = line.strip()
    if stripped.startswith("|"):
        stripped = stripped[1:]
    if stripped.endswith("|") and not stripped.endswith("\\|"):
        stripped = stripped[:-1]

    cells: list[str] = []
    buf: list[str] = []
    in_code = False
    ticks = 0
    i = 0
    n = len(stripped)
    while i < n:
        ch = stripped[i]
        if ch == "\\" and i + 1 < n and stripped[i + 1] == "|":
            buf.append("|")
            i += 2
            continue
        if ch == "`":
            run = 1
            while i + run < n and stripped[i + run] == "`":
                run += 1
            if not in_code:
                in_code = True
                ticks = run
            elif run == ticks:
                in_code = False
                ticks = 0
            buf.append("`" * run)
            i += run
            continue
        if ch == "|" and not in_code:
            cells.append("".join(buf).strip())
            buf = []
            i += 1
            continue
        buf.append(ch)
        i += 1
    cells.append("".join(buf).strip())
    return cells


def _render_fenced_block(lang: str, code_lines: list[str]) -> tuple[str, bool]:
    body = "\n".join(code_lines)
    if lang.lower() == "mermaid":
        return f'<pre class="mermaid">{html.escape(body)}</pre>', True
    lang_attr = f' class="language-{html.escape(lang)}"' if lang else ""
    return f"<pre><code{lang_attr}>{html.escape(body)}</code></pre>", False


def _render_list(
    lines: list[str],
    start: int,
    base_indent: int,
    ordered: bool,
    current_rel: str,
    known_pages: set[str],
    repo_root: Path,
) -> tuple[str, int, bool]:
    tag = "ol" if ordered else "ul"
    pattern = OL_RE if ordered else UL_RE
    items_html: list[str] = []
    has_mermaid = False
    i = start
    n = len(lines)

    while i < n:
        m = pattern.match(lines[i])
        if not m or len(m.group(1)) != base_indent:
            break
        item_indent = base_indent
        text_lines = [m.group(2)]
        sub_blocks: list[str] = []
        i += 1

        while i < n:
            cur = lines[i]
            cur_s = cur.strip()
            if not cur_s:
                # Look ahead: continue item only if next non-blank line is indented deeper.
                peek = i + 1
                while peek < n and not lines[peek].strip():
                    peek += 1
                if peek < n:
                    peek_indent = len(lines[peek]) - len(lines[peek].lstrip(" "))
                    if peek_indent > item_indent:
                        i += 1
                        continue
                break

            cur_indent = len(cur) - len(cur.lstrip(" "))
            if cur_indent <= item_indent:
                break

            fence_m = FENCE_RE.match(cur)
            if fence_m:
                if text_lines:
                    sub_blocks.append(
                        f"<p>{render_inline(join_wrapped_lines(text_lines), current_rel, known_pages, repo_root=repo_root)}</p>"
                    )
                    text_lines = []
                fence_str = fence_m.group(2)
                lang = fence_m.group(3).strip()
                fence_indent = len(fence_m.group(1))
                code_lines: list[str] = []
                i += 1
                while i < n:
                    if lines[i].strip().startswith(fence_str):
                        i += 1
                        break
                    line_val = lines[i]
                    if line_val.startswith(" " * fence_indent):
                        line_val = line_val[fence_indent:]
                    code_lines.append(line_val)
                    i += 1
                block_html, used_mermaid = _render_fenced_block(lang, code_lines)
                has_mermaid = has_mermaid or used_mermaid
                sub_blocks.append(block_html)
                continue

            sub_ul = UL_RE.match(cur)
            sub_ol = OL_RE.match(cur)
            if sub_ul or sub_ol:
                if text_lines:
                    sub_blocks.append(
                        render_inline(
                            join_wrapped_lines(text_lines),
                            current_rel,
                            known_pages,
                            repo_root=repo_root,
                        )
                    )
                    text_lines = []
                sub_indent = len((sub_ul or sub_ol).group(1))
                sub_html, i, used_mermaid = _render_list(
                    lines,
                    i,
                    sub_indent,
                    bool(sub_ol),
                    current_rel,
                    known_pages,
                    repo_root,
                )
                has_mermaid = has_mermaid or used_mermaid
                sub_blocks.append(sub_html)
                continue

            text_lines.append(cur_s)
            i += 1

        if sub_blocks:
            parts = list(sub_blocks)
            if text_lines:
                parts.append(
                    f"<p>{render_inline(join_wrapped_lines(text_lines), current_rel, known_pages, repo_root=repo_root)}</p>"
                )
            items_html.append(f"<li>{''.join(parts)}</li>")
        else:
            rendered = render_inline(
                join_wrapped_lines(text_lines),
                current_rel,
                known_pages,
                repo_root=repo_root,
            )
            items_html.append(f"<li>{rendered}</li>")

    return f"<{tag}>{''.join(items_html)}</{tag}>", i, has_mermaid


def render_markdown(
    md_text: str,
    current_rel: str,
    known_pages: set[str],
    *,
    repo_root: Path = REPO_ROOT,
) -> tuple[str, str, bool]:
    lines = md_text.splitlines()
    out: list[str] = []
    title = Path(current_rel).stem
    slug_counts: Counter[str] = Counter()
    has_mermaid = False
    autolink_inventory = current_rel == "docs/wiki/development/document-inventory.md"
    i = 0
    n = len(lines)

    while i < n:
        line = lines[i]
        stripped = line.strip()
        if not stripped:
            i += 1
            continue

        # Explicit standalone HTML anchor (<a id="..."></a>)
        anchor_m = HTML_ANCHOR_LINE_RE.match(stripped)
        if anchor_m:
            out.append(f'<a id="{html.escape(anchor_m.group(1).strip(), quote=True)}"></a>')
            i += 1
            continue

        # Fenced code block
        fence_m = FENCE_RE.match(line)
        if fence_m and not fence_m.group(1):
            fence_str = fence_m.group(2)
            lang = fence_m.group(3).strip()
            code_lines: list[str] = []
            i += 1
            while i < n and not lines[i].strip().startswith(fence_str):
                code_lines.append(lines[i])
                i += 1
            if i < n:
                i += 1
            block_html, used_mermaid = _render_fenced_block(lang, code_lines)
            has_mermaid = has_mermaid or used_mermaid
            out.append(block_html)
            continue

        # Heading
        heading = HEADING_RE.match(line)
        if heading:
            level = len(heading.group(1))
            raw_title = heading.group(2).strip()
            if level == 1 and title == Path(current_rel).stem:
                title = INLINE_LINK_RE.sub(r"\1", raw_title).replace("`", "")
            base = heading_slug(raw_title)
            count = slug_counts[base]
            slug_counts[base] += 1
            anchor = base if count == 0 else f"{base}-{count}"
            rendered = render_inline(raw_title, current_rel, known_pages, repo_root=repo_root)
            out.append(f'<h{level} id="{html.escape(anchor, quote=True)}">{rendered}</h{level}>')
            i += 1
            continue

        # Horizontal rule
        if stripped in ("---", "***", "___"):
            out.append("<hr />")
            i += 1
            continue

        # Table
        if "|" in line and i + 1 < n and TABLE_SEP_RE.match(lines[i + 1]):
            headers = split_table_row(line)
            i += 2
            rows: list[list[str]] = []
            while i < n and "|" in lines[i] and lines[i].strip():
                rows.append(split_table_row(lines[i]))
                i += 1
            thead = "".join(
                f"<th>{render_inline(cell, current_rel, known_pages, repo_root=repo_root)}</th>"
                for cell in headers
            )
            tbody = "".join(
                "<tr>"
                + "".join(
                    "<td>"
                    + render_inline(
                        cell,
                        current_rel,
                        known_pages,
                        repo_root=repo_root,
                        autolink_known_code_paths=autolink_inventory and col_idx == 0,
                    )
                    + "</td>"
                    for col_idx, cell in enumerate(row)
                )
                + "</tr>"
                for row in rows
            )
            out.append(
                f'<div class="table-wrap"><table><thead><tr>{thead}</tr></thead>'
                f"<tbody>{tbody}</tbody></table></div>"
            )
            continue

        # Blockquote
        if stripped.startswith(">"):
            quote_paras: list[list[str]] = [[]]
            while i < n and lines[i].lstrip().startswith(">"):
                qline = lines[i].lstrip()[1:].lstrip()
                if not qline:
                    if quote_paras[-1]:
                        quote_paras.append([])
                else:
                    quote_paras[-1].append(qline)
                i += 1
            inner = "".join(
                f"<p>{render_inline(join_wrapped_lines(p), current_rel, known_pages, repo_root=repo_root)}</p>"
                for p in quote_paras
                if p
            )
            out.append(f"<blockquote>{inner}</blockquote>")
            continue

        # Unordered or ordered list
        ul_match = UL_RE.match(line)
        ol_match = OL_RE.match(line)
        if ul_match or ol_match:
            base_indent = len((ul_match or ol_match).group(1))
            list_html, i, used_mermaid = _render_list(
                lines,
                i,
                base_indent,
                bool(ol_match),
                current_rel,
                known_pages,
                repo_root,
            )
            has_mermaid = has_mermaid or used_mermaid
            out.append(list_html)
            continue

        # Paragraph
        para_lines: list[str] = []
        while i < n and lines[i].strip():
            cur = lines[i]
            cur_s = cur.strip()
            if (
                HEADING_RE.match(cur)
                or HTML_ANCHOR_LINE_RE.match(cur_s)
                or FENCE_RE.match(cur)
                or cur_s.startswith(">")
                or cur_s in ("---", "***", "___")
                or UL_RE.match(cur)
                or OL_RE.match(cur)
            ):
                break
            if "|" in cur and i + 1 < n and TABLE_SEP_RE.match(lines[i + 1]):
                break
            para_lines.append(cur_s)
            i += 1
        joined = join_wrapped_lines(para_lines)
        out.append(f"<p>{render_inline(joined, current_rel, known_pages, repo_root=repo_root)}</p>")

    return title, "\n".join(out), has_mermaid


def _discover_markdown_files(repo_root: Path) -> set[str]:
    found: set[str] = set()
    for current, dirnames, filenames in os.walk(repo_root):
        dirnames[:] = [d for d in dirnames if d not in IGNORED_DIRS]
        current_path = Path(current)
        for name in filenames:
            if name.endswith(".md"):
                found.add((current_path / name).relative_to(repo_root).as_posix())
    return found


def collect_docs(repo_root: Path = REPO_ROOT) -> list[str]:
    """Collect all repository Markdown docs in logical Chinese sidebar reading order."""
    available = _discover_markdown_files(repo_root)
    ordered: list[str] = []

    def add(rel: str) -> None:
        if rel in available and rel not in ordered:
            ordered.append(rel)

    # 1. Overview
    for rel in ("README.md", "docs/README.md", "docs/wiki/README.md"):
        add(rel)

    # 2. Architecture
    for rel in (
        "docs/wiki/architecture/overview.md",
        "docs/wiki/architecture/client-boundaries.md",
        "docs/wiki/architecture/server-boundaries.md",
    ):
        add(rel)
    for rel in sorted(p for p in available if p.startswith("docs/wiki/architecture/")):
        add(rel)

    # 3. Development
    for rel in sorted(p for p in available if p.startswith("docs/wiki/development/")):
        add(rel)

    # 4. Client mechanisms
    for rel in sorted(
        p
        for p in available
        if p.startswith("docs/wiki/mechanisms/") and Path(p).name.startswith("client-")
    ):
        add(rel)

    # 5. Server mechanisms
    for rel in sorted(
        p
        for p in available
        if p.startswith("docs/wiki/mechanisms/") and Path(p).name.startswith("server-")
    ):
        add(rel)

    # Any other mechanisms
    for rel in sorted(p for p in available if p.startswith("docs/wiki/mechanisms/")):
        add(rel)

    # 6. Ledgers
    for rel in (
        "docs/unsupported-client-features.md",
        "docs/unsupported-server-features.md",
        "docs/client-extensions.md",
    ):
        add(rel)

    # 7. Glossary, templates, decisions, and repository docs
    for rel in (
        "docs/wiki/glossary.md",
        "docs/wiki/mechanism-template.md",
        "docs/wiki/decisions/README.md",
        "AGENTS.md",
        "SECURITY.md",
        "packaging/README.md",
        "scripts/fixtures/mcp-tls/README.md",
    ):
        add(rel)

    for rel in sorted(available):
        add(rel)

    return ordered


def group_label(source_rel: str) -> str:
    if source_rel in ("README.md", "docs/README.md", "docs/wiki/README.md"):
        return "概览"
    if source_rel.startswith("docs/wiki/architecture/"):
        return "架构"
    if source_rel.startswith("docs/wiki/development/"):
        return "开发指南"
    if source_rel.startswith("docs/wiki/mechanisms/"):
        name = Path(source_rel).name
        if name.startswith("client-"):
            return "客户端机制"
        if name.startswith("server-"):
            return "服务端机制"
        return "机制文档"
    if source_rel.startswith("docs/unsupported-") or source_rel == "docs/client-extensions.md":
        return "差异与扩展账本"
    return "术语、模板与规范"


def nav_display_title(source_rel: str, title: str) -> str:
    overrides = {
        "README.md": "项目首页（bbb）",
        "AGENTS.md": "Agent 协作与开发规范（AGENTS.md）",
        "SECURITY.md": "安全漏洞披露策略（SECURITY.md）",
        "packaging/README.md": "发布打包与运维部署指南",
    }
    return overrides.get(source_rel, title)


def build_sidebar(
    current_out: str,
    nav_entries: list[tuple[str, str, str]],
) -> str:
    current_dir = posixpath.dirname(current_out) or "."
    home_href = posixpath.relpath("index.html", current_dir)
    parts: list[str] = [
        '<nav class="sidebar">',
        f'<a class="brand" href="{html.escape(home_href, quote=True)}">bbb 26.1 文档</a>',
        '<div class="ext-links">',
        f'<a href="{REPO_GITHUB_URL}">GitHub 仓库</a>',
        f'<a href="{REPO_GITHUB_URL}/releases">版本发布</a>',
        f'<a href="{REPO_GITHUB_URL}/pkgs/container/bbb">GHCR 镜像</a>',
        "</div>",
    ]
    last_group = ""
    for group, out_rel, label in nav_entries:
        if group != last_group:
            parts.append(f'<div class="group">{html.escape(group)}</div>')
            last_group = group
        href = posixpath.relpath(out_rel, current_dir)
        cls = "nav-item active" if out_rel == current_out else "nav-item"
        escaped_label = html.escape(label)
        parts.append(
            f'<a class="{cls}" href="{html.escape(href, quote=True)}" '
            f'title="{escaped_label}">{escaped_label}</a>'
        )
    parts.append("</nav>")
    return "\n".join(parts)


def _write_directory_redirects(out_dir: Path, rendered_out_rels: set[str]) -> None:
    """Emit index.html redirects for subdirectories that publish README.html."""
    for out_rel in sorted(rendered_out_rels):
        if not out_rel.endswith("/README.html"):
            continue
        dir_index = out_rel[: -len("README.html")] + "index.html"
        if dir_index in rendered_out_rels:
            continue
        dest = out_dir / dir_index
        dest.write_text(
            "<!DOCTYPE html>\n"
            '<html lang="zh-CN">\n'
            "<head>\n"
            '  <meta charset="utf-8" />\n'
            '  <meta http-equiv="refresh" content="0; url=README.html" />\n'
            "  <title>正在跳转 · bbb</title>\n"
            "</head>\n"
            "<body>\n"
            '  <p>正在跳转至 <a href="README.html">README.html</a>…</p>\n'
            "</body>\n"
            "</html>\n",
            encoding="utf-8",
        )


def build_site(out_dir: Path, *, repo_root: Path = REPO_ROOT) -> None:
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / ".nojekyll").write_text("", encoding="utf-8")
    (out_dir / "style.css").write_text(STYLE_CSS + "\n", encoding="utf-8")

    pages = collect_docs(repo_root)
    known_pages = set(pages)

    rendered_pages: list[tuple[str, str, str, str, bool]] = []
    nav_entries: list[tuple[str, str, str]] = []
    for rel in pages:
        text = (repo_root / rel).read_text(encoding="utf-8")
        title, body_html, has_mermaid = render_markdown(
            text, rel, known_pages, repo_root=repo_root
        )
        out_rel = output_rel_for(rel)
        rendered_pages.append((rel, out_rel, title, body_html, has_mermaid))
        nav_entries.append((group_label(rel), out_rel, nav_display_title(rel, title)))

    for rel, out_rel, title, body_html, has_mermaid in rendered_pages:
        dest = out_dir / out_rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        css_href = posixpath.relpath("style.css", posixpath.dirname(out_rel) or ".")
        sidebar = build_sidebar(out_rel, nav_entries)
        github_source = f"{REPO_GITHUB_URL}/blob/main/{rel}"
        page_meta = (
            '<div class="page-meta">'
            f"<span>源文件：<code>{html.escape(rel)}</code></span>"
            f'<a href="{html.escape(github_source, quote=True)}">在 GitHub 查看源码</a>'
            "</div>"
        )
        mermaid_block = MERMAID_SCRIPT if has_mermaid else ""
        doc_html = (
            "<!DOCTYPE html>\n"
            '<html lang="zh-CN">\n'
            "<head>\n"
            '  <meta charset="utf-8" />\n'
            '  <meta name="viewport" content="width=device-width, initial-scale=1.0" />\n'
            f"  <title>{html.escape(title)} · bbb</title>\n"
            f'  <link rel="stylesheet" href="{html.escape(css_href, quote=True)}" />\n'
            "</head>\n"
            "<body>\n"
            f"{sidebar}\n"
            f'<main class="content">\n{page_meta}\n{body_html}\n</main>\n'
            f"{SIDEBAR_SCROLL_SCRIPT}"
            f"{mermaid_block}"
            "</body>\n"
            "</html>\n"
        )
        dest.write_text(doc_html, encoding="utf-8")

    _write_directory_redirects(out_dir, {item[1] for item in rendered_pages})


HREF_ATTR_RE = re.compile(r'(?:href|src)="([^"]+)"')
ID_ATTR_RE = re.compile(r'id="([^"]+)"')
UNRENDERED_LINK_RE = re.compile(r"\]\([^)\n]+\)")
UNRENDERED_BOLD_RE = re.compile(r"\*\*[^*<\n]{2,80}\*\*")


def verify_site(out_dir: Path) -> list[str]:
    """Verify every page, relative link, heading anchor, and absence of standalone web client."""
    errors: list[str] = []
    if (out_dir / "play").exists():
        errors.append("unexpected standalone web client directory: play/")
    if list(out_dir.rglob("coi-serviceworker.js")):
        errors.append("unexpected coi-serviceworker.js found in Pages output")

    html_files = sorted(out_dir.rglob("*.html"))
    if not html_files:
        return ["no HTML files generated"]

    all_files = {p.relative_to(out_dir).as_posix() for p in out_dir.rglob("*") if p.is_file()}
    ids_by_file: dict[str, set[str]] = {}
    texts_by_file: dict[str, str] = {}
    for path in html_files:
        rel = path.relative_to(out_dir).as_posix()
        content = path.read_text(encoding="utf-8")
        texts_by_file[rel] = content
        ids_by_file[rel] = {html.unescape(i) for i in ID_ATTR_RE.findall(content)}

    for rel, content in texts_by_file.items():
        if "play/index.html" in content or "WebGPU Client" in content:
            errors.append(f"{rel}: contains removed standalone WebGPU Client reference")

        for raw_href in HREF_ATTR_RE.findall(content):
            href = html.unescape(raw_href)
            if href.startswith(("http://", "https://", "mailto:")):
                continue
            path_part, _sep, frag = href.partition("#")
            if not path_part:
                target_rel = rel
            else:
                target_rel = posixpath.normpath(posixpath.join(posixpath.dirname(rel), path_part))
            if target_rel.startswith(".."):
                errors.append(f"{rel}: relative link escapes site root: {href}")
                continue
            if target_rel not in all_files:
                errors.append(f"{rel}: broken internal link {href} -> {target_rel}")
                continue
            if frag:
                target_ids = ids_by_file.get(target_rel, set())
                decoded_frag = unquote(frag)
                if (
                    frag not in target_ids
                    and decoded_frag not in target_ids
                    and decoded_frag.lower() not in {i.lower() for i in target_ids}
                ):
                    errors.append(f"{rel}: broken anchor {href} (missing id in {target_rel})")

        if '<main class="content">' in content:
            main_html = content.split('<main class="content">', 1)[1].split("</main>", 1)[0]
            stripped = re.sub(r"<pre[^>]*>.*?</pre>", "", main_html, flags=re.DOTALL)
            stripped = re.sub(r"<code[^>]*>.*?</code>", "", stripped, flags=re.DOTALL)
            for m in UNRENDERED_LINK_RE.finditer(stripped):
                errors.append(f"{rel}: unrendered Markdown link fragment: {m.group(0)[:80]!r}")
            for m in UNRENDERED_BOLD_RE.finditer(stripped):
                errors.append(f"{rel}: unrendered Markdown bold span: {m.group(0)[:80]!r}")
            if "&lt;a id=" in stripped:
                errors.append(f"{rel}: escaped raw HTML anchor leaked into rendered content")

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Build the static GitHub Pages documentation site for bbb."
    )
    parser.add_argument("--out", type=Path, required=True, help="Output directory for Pages site.")
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Verify all internal links, heading anchors, and rendered HTML after building.",
    )
    args = parser.parse_args(argv)
    build_site(args.out)
    if args.verify:
        errors = verify_site(args.out)
        if errors:
            for err in errors:
                print(f"pages verify error: {err}")
            return 1
        print(f"Verified GitHub Pages site in {args.out} (0 link/anchor errors)")
    print(f"Built GitHub Pages site in {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
