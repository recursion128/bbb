#!/usr/bin/env python3
"""Cross-platform P0 allowlist classification audit for crates/."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

HIT_RE = re.compile(r"residual|fallback|unsupported")


def _compile_posix_regex(pattern: str) -> re.Pattern[str]:
    translated = pattern.replace("[:space:]", r" \t\r\n\f\v").replace("[:alnum:]", "A-Za-z0-9")
    return re.compile(translated)


def load_allowlist(path: Path) -> list[tuple[re.Pattern[str], re.Pattern[str]]]:
    if not path.is_file():
        print(f"P0 audit allowlist not found: {path}", file=sys.stderr)
        raise SystemExit(2)

    rules: list[tuple[re.Pattern[str], re.Pattern[str]]] = []
    invalid = False
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        if "\t" not in raw:
            print(
                f"invalid P0 audit allowlist entry at line {lineno}: "
                "expected <path-regex><TAB><line-regex>",
                file=sys.stderr,
            )
            invalid = True
            continue
        path_pat, line_pat = raw.split("\t", 1)
        if not path_pat or not line_pat:
            print(
                f"invalid P0 audit allowlist entry at line {lineno}: regexes must be non-empty",
                file=sys.stderr,
            )
            invalid = True
            continue
        try:
            rules.append((_compile_posix_regex(path_pat), _compile_posix_regex(line_pat)))
        except re.error as exc:
            print(
                f"invalid P0 audit allowlist regex at line {lineno}: {exc}",
                file=sys.stderr,
            )
            invalid = True

    if invalid:
        raise SystemExit(2)
    return rules


def candidate_files(repo_root: Path) -> list[Path]:
    crates_dir = repo_root / "crates"
    if not crates_dir.is_dir():
        return []
    git_ls = subprocess.run(
        ["git", "-C", str(repo_root), "ls-files", "-co", "--exclude-standard", "crates"],
        check=False,
        capture_output=True,
        text=True,
    )
    if git_ls.returncode == 0:
        files: list[Path] = []
        for rel in git_ls.stdout.splitlines():
            if not rel or "/target/" in f"/{rel}/":
                continue
            full = repo_root / rel
            if full.is_file():
                files.append(full)
        return sorted(files)
    return sorted(
        p for p in crates_dir.rglob("*") if p.is_file() and "target" not in p.parts
    )


def run_audit(repo_root: Path, allowlist_path: Path | None = None) -> int:
    allowlist = allowlist_path or (repo_root / "scripts" / "data" / "p0-audit-allowlist.txt")
    rules = load_allowlist(allowlist)

    hit_count = 0
    unclassified = 0
    for file_path in candidate_files(repo_root):
        rel = file_path.relative_to(repo_root).as_posix()
        try:
            text = file_path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for lineno, content in enumerate(text.splitlines(), 1):
            if not HIT_RE.search(content):
                continue
            hit_count += 1
            if not any(pr.search(rel) and lr.search(content) for pr, lr in rules):
                print(f"{rel}:{lineno}:{content}")
                unclassified += 1

    if unclassified:
        return 1
    print(f"P0 audit: {hit_count} hits, all classified against {len(rules)} allowlist entries")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
    )
    parser.add_argument("--allowlist", type=Path)
    args = parser.parse_args(argv)
    return run_audit(args.repo_root.resolve(), args.allowlist)


if __name__ == "__main__":
    raise SystemExit(main())
