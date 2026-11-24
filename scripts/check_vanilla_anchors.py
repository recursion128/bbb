#!/usr/bin/env python3
"""Open every Vanilla source anchor a change introduces and report the bad ones.

An anchor is a Java file name followed by a colon and either one line number or
a hyphenated range.

The repository's discipline is that a comment citing Vanilla source *is* the
proof: a constant, an order or a branch is justified by the line range beside
it. That only works while the range is real, and a line number is the one part
of a comment nothing re-reads. Anchors that name a file that does not exist,
a range past the end of it, or a range whose first or last line is blank or a
bare `@Override` have all been written by hand and by agents alike.

This opens each one and prints its first and last line, so a reviewer reads
what the anchor actually points at rather than trusting it. Three shapes are
reported as problems:

* the file is missing from the prepared Vanilla source tree;
* the range runs past the end of every candidate file with that name;
* the first or last line is blank, or is exactly `@Override` -- an anchor whose
  edge is an annotation names the annotation rather than the member.

Anchors that are not `.java` (a language file, a JSON registry entry) are not
matched and still have to be read by hand.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import zipfile
from collections.abc import Sequence
from pathlib import Path

#: A Java file name, a colon, and one line number or a hyphenated range --
#: the two spellings the tree uses.
ANCHOR = re.compile(r"([A-Za-z0-9_$]+\.java):(\d+)(?:-(\d+))?")
PACKAGE_PREFIX = re.compile(r"((?:[A-Za-z0-9_]+/)+)\Z")
ALL_SCAN_EXCLUDES = (
    ":!scripts/tests/test_check_vanilla_anchors.py",
    ":!scripts/tests/test_vanilla_java_source.py",
)


def source_roots(repo_root: Path, *, environ: dict[str, str] | None = None) -> list[Path]:
    """Return prepared Vanilla source directories without probing ad-hoc home paths."""
    roots: list[Path] = []
    script_dir = str(repo_root / "scripts")
    if script_dir not in sys.path:
        sys.path.insert(0, script_dir)
    try:
        from bbb_tools.vanilla.errors import VanillaError
        from bbb_tools.vanilla.workspace import VanillaWorkspace

        workspace = VanillaWorkspace.resolve("26.1", environ=environ)
        roots.append(workspace.require_prepared().sources)
    except (ImportError, VanillaError):
        pass
    return [root for root in roots if root.is_dir()]


def index_by_name(roots: list[Path]) -> dict[str, list[Path]]:
    """Java files by base name.

    A base name is not unique -- 26.1 has two `EntitySelector.java`, one of
    them 70 lines long -- so every candidate is kept and each is tried.
    """
    index: dict[str, list[Path]] = {}
    for root in roots:
        for dirpath, _dirnames, filenames in os.walk(root):
            base = Path(dirpath)
            for filename in filenames:
                if filename.endswith(".java"):
                    index.setdefault(filename, []).append(base / filename)
    return index


def _git_stdout(repo_root: Path, *args: str) -> str | None:
    result = subprocess.run(
        ["git", "-C", str(repo_root), *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if result.returncode != 0:
        return None
    return result.stdout.strip()


def resolve_committed_base(repo_root: Path) -> str | None:
    """Choose merge-base with main (or HEAD~1 when already at/without main)."""
    head = _git_stdout(repo_root, "rev-parse", "--verify", "--quiet", "HEAD")
    if not head:
        return None
    for candidate in ("main", "refs/remotes/origin/main"):
        if _git_stdout(repo_root, "rev-parse", "--verify", "--quiet", candidate):
            merge_base = _git_stdout(repo_root, "merge-base", "HEAD", candidate)
            if merge_base and merge_base != head:
                return merge_base
            break
    parent = _git_stdout(repo_root, "rev-parse", "--verify", "--quiet", "HEAD~1")
    return parent or None


def _pathspecs(paths: list[str]) -> list[str]:
    return list(paths) if paths else [".", *ALL_SCAN_EXCLUDES]


def _diff_added_lines(
    repo_root: Path, diff_target: Sequence[str], paths: list[str]
) -> tuple[bool, list[str]]:
    diff = subprocess.run(
        [
            "git",
            "-C",
            str(repo_root),
            "diff",
            *diff_target,
            "--unified=0",
            "--",
            *_pathspecs(paths),
        ],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=True,
    ).stdout
    has_diff = bool(diff.strip())
    added = [
        line[1:]
        for line in diff.splitlines()
        if line.startswith("+") and not line.startswith("+++")
    ]
    return has_diff, added


def index_library_classes(roots: Sequence[Path]) -> set[str]:
    """Java `.class` names shipped in prepared dependency JARs (e.g. Brigadier)."""
    classes: set[str] = set()
    for root in roots:
        # `sources/<version>` sits beside `downloads/<version>/libraries`.
        version = root.name
        candidate_dirs = [
            root.parents[1] / "downloads" / version / "libraries"
            if len(root.parents) >= 2
            else None,
            root.parent / "libraries",
        ]
        for lib_dir in candidate_dirs:
            if lib_dir is None or not lib_dir.is_dir():
                continue
            for jar_path in lib_dir.rglob("*.jar"):
                try:
                    with zipfile.ZipFile(jar_path) as archive:
                        for entry in archive.namelist():
                            if not entry.endswith(".class"):
                                continue
                            stem = Path(entry).stem
                            if stem in ("module-info", "package-info"):
                                continue
                            classes.add(f"{stem}.java")
                except (OSError, zipfile.BadZipFile):
                    continue
    return classes


def package_hint_before(line: str, match_start: int) -> str | None:
    match = PACKAGE_PREFIX.search(line[:match_start])
    if match is None:
        return None
    return match.group(1).strip("/")


def _all_repo_lines(repo_root: Path, paths: list[str]) -> list[str]:
    pathspecs = _pathspecs(paths)
    result = subprocess.run(
        [
            "git",
            "-C",
            str(repo_root),
            "grep",
            "--untracked",
            "--no-color",
            "-h",
            "-E",
            r"[A-Za-z0-9_$]+\.java:[0-9]+",
            "--",
            *pathspecs,
        ],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if result.returncode in (0, 1):
        return result.stdout.splitlines()

    lines: list[str] = []
    targets = [repo_root / p for p in paths] if paths else [repo_root]
    excluded = {
        (repo_root / "scripts/tests/test_check_vanilla_anchors.py").resolve(),
        (repo_root / "scripts/tests/test_vanilla_java_source.py").resolve(),
    }
    for base in targets:
        iter_paths = [base] if base.is_file() else base.rglob("*")
        for path in iter_paths:
            if not path.is_file() or path.resolve() in excluded:
                continue
            if ".git" in path.parts:
                continue
            try:
                lines.extend(path.read_text(encoding="utf-8").splitlines())
            except (OSError, UnicodeDecodeError):
                continue
    return lines


def added_lines(
    repo_root: Path, base: str | None, paths: list[str], *, check_all: bool = False
) -> list[str]:
    """The lines a change adds, falling back to full-repo scan when diff has no anchors."""
    if check_all:
        return _all_repo_lines(repo_root, paths)

    if base is not None:
        _, lines = _diff_added_lines(repo_root, [base], paths)
        lines.extend(_untracked_lines(repo_root, paths))
        return lines

    has_head_diff, lines = _diff_added_lines(repo_root, ["HEAD"], paths)
    untracked = _untracked_lines(repo_root, paths)
    if has_head_diff or untracked:
        lines.extend(untracked)
        return lines

    committed_base = resolve_committed_base(repo_root)
    if committed_base is None:
        return _all_repo_lines(repo_root, paths)
    _, committed_lines = _diff_added_lines(repo_root, [committed_base, "HEAD"], paths)
    if not any(ANCHOR.search(line) for line in committed_lines):
        return _all_repo_lines(repo_root, paths)
    return committed_lines


def _untracked_lines(repo_root: Path, paths: list[str]) -> list[str]:
    untracked = subprocess.run(
        [
            "git",
            "-C",
            str(repo_root),
            "ls-files",
            "--others",
            "--exclude-standard",
            "--",
            *_pathspecs(paths),
        ],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=True,
    ).stdout.splitlines()
    lines: list[str] = []
    for relative in untracked:
        if not relative:
            continue
        path = repo_root / relative
        try:
            lines.extend(path.read_text(encoding="utf-8").splitlines())
        except (OSError, UnicodeDecodeError):
            continue
    return lines


def _read_cached_lines(
    candidate: Path, file_cache: dict[Path, list[str]] | None
) -> list[str] | None:
    if file_cache is not None and candidate in file_cache:
        return file_cache[candidate]
    try:
        lines = candidate.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return None
    if file_cache is not None:
        file_cache[candidate] = lines
    return lines


def resolve(
    index: dict[str, list[Path]],
    name: str,
    first: int,
    last: int,
    *,
    package_hint: str | None = None,
    file_cache: dict[Path, list[str]] | None = None,
) -> tuple[Path | None, list[str] | None]:
    """The matching candidate file that contains the whole range, with its lines."""
    candidates = index.get(name, [])
    if package_hint and len(candidates) > 1:
        suffix = f"/{package_hint}/{name}"
        direct = f"{package_hint}/{name}"
        narrowed = [
            candidate
            for candidate in candidates
            if candidate.as_posix().endswith(suffix) or candidate.as_posix().endswith(direct)
        ]
        if narrowed:
            candidates = narrowed

    fallback: tuple[Path, list[str]] | None = None
    for candidate in candidates:
        lines = _read_cached_lines(candidate, file_cache)
        if lines is None:
            continue
        if 1 <= first <= last <= len(lines):
            if not problems_with(lines[first - 1], lines[last - 1]):
                return candidate, lines
            if fallback is None:
                fallback = (candidate, lines)
    return fallback if fallback is not None else (None, None)


def problems_with(first_line: str, last_line: str) -> list[str]:
    found = []
    if not first_line.strip():
        found.append("first line is blank")
    if not last_line.strip():
        found.append("last line is blank")
    if first_line.strip() == "@Override":
        found.append("first line is @Override")
    if last_line.strip() == "@Override":
        found.append("last line is @Override")
    return found


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", default=".")
    parser.add_argument(
        "--base",
        default=None,
        help="git base ref to diff against (defaults to HEAD, or merge-base with main / HEAD~1 when clean)",
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="scan all tracked/untracked repository anchors instead of only the current diff",
    )
    parser.add_argument("paths", nargs="*", help="restrict to these paths")
    args = parser.parse_args(argv)

    repo_root = Path(args.repo_root).resolve()
    roots = source_roots(repo_root)
    if not roots:
        if os.environ.get("BBB_REQUIRE_VANILLA") == "1":
            print("no Vanilla source tree is available; prepare it with cargo dev vanilla prepare 26.1")
            return 2
        print("anchors: skipped (no prepared Vanilla 26.1 source tree; set BBB_REQUIRE_VANILLA=1 to enforce)")
        return 0
    index = index_by_name(roots)
    lib_classes: set[str] | None = None
    file_cache: dict[Path, list[str]] = {}

    seen: set[tuple[str | None, str, int, int]] = set()
    library_skipped = 0
    bad = 0
    for line in added_lines(repo_root, args.base, args.paths, check_all=args.all):
        for match in ANCHOR.finditer(line):
            name = match.group(1)
            first = int(match.group(2))
            last = int(match.group(3) or match.group(2))
            pkg_hint = package_hint_before(line, match.start())
            key = (pkg_hint, name, first, last)
            if key in seen:
                continue
            seen.add(key)
            if name not in index:
                if lib_classes is None:
                    lib_classes = index_library_classes(roots)
                if name in lib_classes:
                    library_skipped += 1
                    continue
                print(f"MISSING FILE            {name}:{first}-{last}")
                bad += 1
                continue
            path, lines = resolve(
                index,
                name,
                first,
                last,
                package_hint=pkg_hint,
                file_cache=file_cache,
            )
            if path is None or lines is None:
                sizes = ", ".join(
                    str(len(_read_cached_lines(candidate, file_cache) or ()))
                    for candidate in index[name]
                )
                label = f"{pkg_hint}/{name}" if pkg_hint else name
                print(f"OUT OF RANGE            {label}:{first}-{last} (candidate lengths: {sizes})")
                bad += 1
                continue
            found = problems_with(lines[first - 1], lines[last - 1])
            status = "; ".join(found) if found else "ok"
            if found or not args.all:
                label = f"{pkg_hint}/{name}" if pkg_hint else name
                print(
                    f"{status:23} {label}:{first}-{last}  "
                    f"|{lines[first - 1].strip()[:64]}| .. |{lines[last - 1].strip()[:64]}|"
                )
            bad += bool(found)

    print()
    lib_note = f" ({library_skipped} library skipped)" if library_skipped else ""
    print(f"{len(seen)} anchors checked{lib_note}, {bad} problems")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
