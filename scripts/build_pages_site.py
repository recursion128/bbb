"""Build the static GitHub Pages site (Chinese Wiki docs + WebGPU client with COOP/COEP SW)."""

from __future__ import annotations

import argparse
import html
import posixpath
import re
import shutil
import unicodedata
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$")
UL_RE = re.compile(r"^(\s*)[-*+]\s+(.*)$")
OL_RE = re.compile(r"^(\s*)\d+\.\s+(.*)$")
TABLE_SEP_RE = re.compile(r"^\s*\|?(?:\s*:?-{3,}:?\s*\|)+\s*:?-{3,}:?\s*\|?\s*$")
LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
CODE_SPAN_RE = re.compile(r"`([^`\n]+)`")
BOLD_RE = re.compile(r"\*\*([^*\n]+)\*\*")
ITALIC_RE = re.compile(r"(?<!\*)\*([^*\n]+)\*(?!\*)")

COI_SERVICE_WORKER_JS = """/* Self-contained Cross-Origin Isolation Service Worker for GitHub Pages.
 * Adds COOP/COEP headers to same-origin responses so SharedArrayBuffer works
 * without custom server response headers.
 */
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.cache === "only-if-cached" && req.mode !== "same-origin") {
    return;
  }
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) {
    return;
  }
  event.respondWith(
    fetch(req)
      .then((response) => {
        if (response.status === 0) {
          return response;
        }
        const headers = new Headers(response.headers);
        headers.set("Cross-Origin-Embedder-Policy", "require-corp");
        headers.set("Cross-Origin-Opener-Policy", "same-origin");
        headers.set("Cross-Origin-Resource-Policy", "same-origin");
        return new Response(response.body, {
          status: response.status,
          statusText: response.statusText,
          headers,
        });
      })
      .catch((err) => {
        console.error("[coi-sw] fetch failed:", err);
        throw err;
      })
  );
});
"""

COI_BOOTSTRAP_SNIPPET = """  <script>
    (function () {
      if (window.crossOriginIsolated || !("serviceWorker" in navigator)) {
        return;
      }
      const reloadKey = "bbb_coi_sw_reloaded";
      navigator.serviceWorker.register("./coi-serviceworker.js", { scope: "./" }).then(
        (registration) => {
          if (registration.active && !navigator.serviceWorker.controller) {
            if (!sessionStorage.getItem(reloadKey)) {
              sessionStorage.setItem(reloadKey, "1");
              location.reload();
            }
            return;
          }
          const worker = registration.installing || registration.waiting;
          if (worker) {
            worker.addEventListener("statechange", () => {
              if (worker.state === "activated" && !sessionStorage.getItem(reloadKey)) {
                sessionStorage.setItem(reloadKey, "1");
                location.reload();
              }
            });
          }
        },
        (err) => console.warn("COI service worker registration failed:", err)
      );
    })();
  </script>
"""

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
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", sans-serif;
  line-height: 1.65;
  display: flex;
  min-height: 100vh;
}
nav.sidebar {
  width: 280px;
  flex-shrink: 0;
  background: var(--panel);
  border-right: 1px solid var(--border);
  padding: 20px 16px;
  overflow-y: auto;
  position: sticky;
  top: 0;
  height: 100vh;
}
nav.sidebar h2 {
  margin: 0 0 12px;
  font-size: 16px;
  letter-spacing: 0.03em;
}
nav.sidebar .group {
  margin: 14px 0 6px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}
nav.sidebar a {
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
nav.sidebar a:hover, nav.sidebar a.active {
  background: #21262d;
  color: var(--accent);
}
nav.sidebar .cta {
  display: block;
  margin: 10px 0 14px;
  padding: 8px 12px;
  border-radius: 6px;
  background: #1f6feb;
  color: #ffffff;
  text-align: center;
  font-weight: 600;
}
nav.sidebar .cta:hover { background: #388bfd; color: #ffffff; }
main.content {
  flex: 1;
  min-width: 0;
  max-width: 960px;
  padding: 32px 40px 64px;
}
h1, h2, h3, h4 {
  margin-top: 1.5em;
  margin-bottom: 0.6em;
  border-bottom: 1px solid var(--border);
  padding-bottom: 0.25em;
}
h3, h4 { border-bottom: 0; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
code {
  background: var(--code-bg);
  padding: 0.15em 0.4em;
  border-radius: 4px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.9em;
}
pre {
  background: var(--code-bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 14px 16px;
  overflow-x: auto;
}
pre code { background: transparent; padding: 0; }
table {
  border-collapse: collapse;
  width: 100%;
  margin: 16px 0;
  font-size: 14px;
}
th, td {
  border: 1px solid var(--border);
  padding: 8px 10px;
  text-align: left;
  vertical-align: top;
}
th { background: var(--panel); }
blockquote {
  margin: 14px 0;
  padding: 6px 14px;
  border-left: 3px solid var(--accent);
  color: var(--muted);
  background: rgba(88, 166, 255, 0.06);
}
@media (max-width: 860px) {
  body { flex-direction: column; }
  nav.sidebar { width: 100%; height: auto; position: static; border-right: 0; border-bottom: 1px solid var(--border); }
  main.content { padding: 20px; }
}
""".strip()


def heading_slug(text: str) -> str:
    plain = LINK_RE.sub(r"\1", text).replace("`", "").replace("*", "").replace("~", "")
    out: list[str] = []
    for ch in plain.strip().lower():
        if ch == " ":
            out.append("-")
        elif ch in ("-", "_"):
            out.append(ch)
        elif unicodedata.category(ch)[0] in ("L", "N"):
            out.append(ch)
    return "".join(out) or "section"


def rewrite_link(raw_target: str, current_rel: str, known_pages: set[str]) -> str:
    if raw_target.startswith(("http://", "https://", "mailto:", "#")):
        return raw_target
    path_part, sep, frag = raw_target.partition("#")
    if not path_part:
        return raw_target
    current_dir = posixpath.dirname(current_rel)
    resolved = posixpath.normpath(posixpath.join(current_dir, path_part))
    if resolved in known_pages:
        target_html = resolved[:-3] + ".html" if resolved.endswith(".md") else resolved
        if resolved == "README.md":
            target_html = "index.html"
        rel = posixpath.relpath(target_html, current_dir or ".")
        return f"{rel}{sep}{frag}"
    # Non-rendered repo files (e.g. Cargo.toml, crates/...) link to GitHub blob view.
    return f"https://github.com/recursion128/bbb/blob/main/{resolved}{sep}{frag}"


def render_inline(text: str, current_rel: str, known_pages: set[str]) -> str:
    tokens: list[str] = []

    def stash_code(match: re.Match[str]) -> str:
        idx = len(tokens)
        tokens.append(f"<code>{html.escape(match.group(1))}</code>")
        return f"\x00CODE{idx}\x00"

    protected = CODE_SPAN_RE.sub(stash_code, text)
    escaped = html.escape(protected)

    def replace_link(match: re.Match[str]) -> str:
        label = match.group(1)
        href = html.unescape(match.group(2))
        rewritten = rewrite_link(href, current_rel, known_pages)
        return f'<a href="{html.escape(rewritten, quote=True)}">{label}</a>'

    escaped = LINK_RE.sub(replace_link, escaped)
    escaped = BOLD_RE.sub(r"<strong>\1</strong>", escaped)
    escaped = ITALIC_RE.sub(r"<em>\1</em>", escaped)
    for idx, snippet in enumerate(tokens):
        escaped = escaped.replace(f"\x00CODE{idx}\x00", snippet)
    return escaped


def split_table_row(line: str) -> list[str]:
    stripped = line.strip()
    if stripped.startswith("|"):
        stripped = stripped[1:]
    if stripped.endswith("|"):
        stripped = stripped[:-1]
    return [cell.strip() for cell in stripped.split("|")]


def render_markdown(md_text: str, current_rel: str, known_pages: set[str]) -> tuple[str, str]:
    lines = md_text.splitlines()
    out: list[str] = []
    title = Path(current_rel).stem
    slug_counts: Counter[str] = Counter()
    i = 0
    n = len(lines)

    while i < n:
        line = lines[i]
        stripped = line.strip()
        if not stripped:
            i += 1
            continue

        # Fenced code block
        if stripped.startswith(("```", "~~~")):
            fence = stripped[:3]
            lang = stripped[3:].strip()
            code_lines: list[str] = []
            i += 1
            while i < n and not lines[i].strip().startswith(fence):
                code_lines.append(lines[i])
                i += 1
            if i < n:
                i += 1
            lang_attr = f' class="language-{html.escape(lang)}"' if lang else ""
            out.append(f"<pre><code{lang_attr}>{html.escape(chr(10).join(code_lines))}</code></pre>")
            continue

        # Heading
        heading = HEADING_RE.match(line)
        if heading:
            level = len(heading.group(1))
            raw_title = heading.group(2).strip()
            if level == 1 and title == Path(current_rel).stem:
                title = LINK_RE.sub(r"\1", raw_title).replace("`", "")
            base = heading_slug(raw_title)
            count = slug_counts[base]
            slug_counts[base] += 1
            anchor = base if count == 0 else f"{base}-{count}"
            rendered = render_inline(raw_title, current_rel, known_pages)
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
                f"<th>{render_inline(cell, current_rel, known_pages)}</th>" for cell in headers
            )
            tbody = "".join(
                "<tr>"
                + "".join(
                    f"<td>{render_inline(cell, current_rel, known_pages)}</td>" for cell in row
                )
                + "</tr>"
                for row in rows
            )
            out.append(f"<table><thead><tr>{thead}</tr></thead><tbody>{tbody}</tbody></table>")
            continue

        # Blockquote
        if stripped.startswith(">"):
            quote_lines: list[str] = []
            while i < n and lines[i].lstrip().startswith(">"):
                quote_lines.append(lines[i].lstrip()[1:].lstrip())
                i += 1
            inner = "<br />".join(
                render_inline(q, current_rel, known_pages) for q in quote_lines if q
            )
            out.append(f"<blockquote>{inner}</blockquote>")
            continue

        # Unordered or ordered list
        ul_match = UL_RE.match(line)
        ol_match = OL_RE.match(line)
        if ul_match or ol_match:
            tag = "ul" if ul_match else "ol"
            pattern = UL_RE if ul_match else OL_RE
            items: list[str] = []
            while i < n:
                m = pattern.match(lines[i])
                if not m:
                    break
                items.append(f"<li>{render_inline(m.group(2), current_rel, known_pages)}</li>")
                i += 1
            out.append(f"<{tag}>{''.join(items)}</{tag}>")
            continue

        # Paragraph
        para: list[str] = []
        while i < n and lines[i].strip():
            cur = lines[i]
            cur_s = cur.strip()
            if (
                HEADING_RE.match(cur)
                or cur_s.startswith(("```", "~~~", ">"))
                or cur_s in ("---", "***", "___")
                or UL_RE.match(cur)
                or OL_RE.match(cur)
            ):
                break
            if "|" in cur and i + 1 < n and TABLE_SEP_RE.match(lines[i + 1]):
                break
            para.append(render_inline(cur_s, current_rel, known_pages))
            i += 1
        out.append(f"<p>{' '.join(para)}</p>")

    return title, "\n".join(out)


def collect_docs() -> list[str]:
    pages = ["README.md"]
    for extra in (
        "docs/wiki/README.md",
        "docs/unsupported-client-features.md",
        "docs/unsupported-server-features.md",
        "docs/client-extensions.md",
    ):
        if (REPO_ROOT / extra).is_file():
            pages.append(extra)
    wiki_dir = REPO_ROOT / "docs" / "wiki"
    if wiki_dir.is_dir():
        for path in sorted(wiki_dir.rglob("*.md")):
            rel = path.relative_to(REPO_ROOT).as_posix()
            if rel not in pages:
                pages.append(rel)
    return pages


def output_rel_for(source_rel: str) -> str:
    if source_rel == "README.md":
        return "index.html"
    return source_rel[:-3] + ".html"


def build_sidebar(
    current_out: str,
    nav_entries: list[tuple[str, str, str]],
    has_web_play: bool,
) -> str:
    current_dir = posixpath.dirname(current_out) or "."
    parts: list[str] = ['<nav class="sidebar">', "<h2>bbb 26.1 Docs</h2>"]
    if has_web_play:
        play_href = posixpath.relpath("play/index.html", current_dir)
        parts.append(f'<a class="cta" href="{html.escape(play_href, quote=True)}">WebGPU Client</a>')
    parts.append('<a href="https://github.com/recursion128/bbb/releases">Releases</a>')
    parts.append('<a href="https://github.com/recursion128/bbb/pkgs/container/bbb">GHCR Container</a>')
    last_group = ""
    for group, out_rel, label in nav_entries:
        if group != last_group:
            parts.append(f'<div class="group">{html.escape(group)}</div>')
            last_group = group
        href = posixpath.relpath(out_rel, current_dir)
        cls = ' class="active"' if out_rel == current_out else ""
        parts.append(f'<a{cls} href="{html.escape(href, quote=True)}">{html.escape(label)}</a>')
    parts.append("</nav>")
    return "\n".join(parts)


def group_label(source_rel: str) -> str:
    if source_rel in ("README.md", "docs/wiki/README.md"):
        return "Overview"
    if source_rel.startswith("docs/unsupported-") or source_rel == "docs/client-extensions.md":
        return "Ledgers"
    parts = source_rel.split("/")
    if len(parts) >= 4 and parts[:2] == ["docs", "wiki"]:
        return parts[2].capitalize()
    return "Wiki"


def stage_web_play(web_dist: Path, out_dir: Path) -> bool:
    index_src = web_dist / "index.html"
    if not index_src.is_file():
        return False
    play_dir = out_dir / "play"
    if play_dir.exists():
        shutil.rmtree(play_dir)
    shutil.copytree(web_dist, play_dir)
    (play_dir / "coi-serviceworker.js").write_text(COI_SERVICE_WORKER_JS, encoding="utf-8")
    index_html = (play_dir / "index.html").read_text(encoding="utf-8")
    if "coi-serviceworker.js" not in index_html:
        marker = '<script type="module">'
        if marker in index_html:
            index_html = index_html.replace(marker, COI_BOOTSTRAP_SNIPPET + "  " + marker, 1)
        else:
            index_html = index_html.replace("</head>", COI_BOOTSTRAP_SNIPPET + "</head>", 1)
        (play_dir / "index.html").write_text(index_html, encoding="utf-8")
    return True


def build_site(out_dir: Path, web_dist: Path | None) -> None:
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / ".nojekyll").write_text("", encoding="utf-8")
    (out_dir / "style.css").write_text(STYLE_CSS + "\n", encoding="utf-8")

    has_web_play = stage_web_play(web_dist, out_dir) if web_dist is not None else False
    pages = collect_docs()
    known_pages = set(pages)

    rendered_pages: list[tuple[str, str, str, str]] = []
    nav_entries: list[tuple[str, str, str]] = []
    for rel in pages:
        text = (REPO_ROOT / rel).read_text(encoding="utf-8")
        title, body_html = render_markdown(text, rel, known_pages)
        out_rel = output_rel_for(rel)
        rendered_pages.append((rel, out_rel, title, body_html))
        nav_entries.append((group_label(rel), out_rel, title))

    for _rel, out_rel, title, body_html in rendered_pages:
        dest = out_dir / out_rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        css_href = posixpath.relpath("style.css", posixpath.dirname(out_rel) or ".")
        sidebar = build_sidebar(out_rel, nav_entries, has_web_play)
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
            f'<main class="content">\n{body_html}\n</main>\n'
            "</body>\n"
            "</html>\n"
        )
        dest.write_text(doc_html, encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build static GitHub Pages site for bbb.")
    parser.add_argument("--out", type=Path, required=True, help="Output directory for Pages site.")
    parser.add_argument(
        "--web-dist",
        type=Path,
        default=None,
        help="Optional built WASM web-dist directory to embed at /play/ with COOP/COEP SW.",
    )
    args = parser.parse_args(argv)
    build_site(args.out, args.web_dist)
    print(f"Built GitHub Pages site in {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
