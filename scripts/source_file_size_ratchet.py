"""Source-file size ratchet for every hand-written crate.

A file that several lanes must edit at once is a conflict magnet, and a file
nobody can read in one sitting is a review hazard, so hand-written Rust
sources in every crate carry a line cap. Files that were already
over the cap when the rule landed are pinned in the ratchet data file with the
line count they may not exceed; every split lowers or drops the pin, nothing
raises it. Generated sources (``// @generated`` header) are exempt: their size
is the size of the Vanilla fact they carry.

Usage:
    source_file_size_ratchet.py check    exit 0 when every file is within its
                                         bound, 1 with the offenders listed
    source_file_size_ratchet.py update   rewrite the data file: drop pins that
                                         are no longer needed, lower pins to
                                         the current count, never raise one,
                                         add no new pin (a new offender must be
                                         split, not pinned)
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
DATA_PATH = Path("scripts/data/source-file-size-ratchet.json")
GENERATED_MARKER = "// @generated"
SCHEMA_VERSION = 1


def load_data(repo_root: Path) -> dict:
    return json.loads((repo_root / DATA_PATH).read_text(encoding="utf-8"))


def is_generated(path: Path) -> bool:
    with path.open(encoding="utf-8", errors="replace") as handle:
        return handle.readline().startswith(GENERATED_MARKER)


def count_lines(path: Path) -> int:
    with path.open("rb") as handle:
        return sum(1 for _ in handle)


def scan(repo_root: Path, data: dict) -> dict[str, int]:
    """Line counts for every hand-written Rust file under the declared roots."""
    counts: dict[str, int] = {}
    for root in data["roots"]:
        for path in sorted((repo_root / root).rglob("*.rs")):
            if is_generated(path):
                continue
            counts[path.relative_to(repo_root).as_posix()] = count_lines(path)
    return counts


def offenders(data: dict, counts: dict[str, int]) -> list[tuple[str, int, int]]:
    cap = data["line_cap"]
    pins = data["pinned"]
    bad = []
    for path, count in sorted(counts.items()):
        bound = pins.get(path, cap)
        if count > bound:
            bad.append((path, count, bound))
    return bad


def stale_pins(data: dict, counts: dict[str, int]) -> list[str]:
    """Pins whose file is gone or already within the cap."""
    cap = data["line_cap"]
    return sorted(path for path in data["pinned"] if path not in counts or counts[path] <= cap)


def check(repo_root: Path) -> int:
    data = load_data(repo_root)
    counts = scan(repo_root, data)
    bad = offenders(data, counts)
    stale = stale_pins(data, counts)
    for path, count, bound in bad:
        pinned = " (pinned)" if path in data["pinned"] else ""
        print(
            f"source size ratchet: {path} has {count} lines, bound {bound}{pinned}",
            file=sys.stderr,
        )
    for path in stale:
        print(
            f"source size ratchet: pin for {path} is stale; run `update` to drop it",
            file=sys.stderr,
        )
    if bad or stale:
        print(
            "source size ratchet fix: split the file into a facade plus domain modules "
            "(moves only, pub(super)); pins only ever shrink",
            file=sys.stderr,
        )
        return 1
    print(
        f"source size ratchet: {len(counts)} hand-written files within the "
        f"{data['line_cap']}-line cap, {len(data['pinned'])} pinned"
    )
    return 0


def update(repo_root: Path) -> int:
    data = load_data(repo_root)
    counts = scan(repo_root, data)
    cap = data["line_cap"]
    pins = {}
    for path, bound in data["pinned"].items():
        count = counts.get(path)
        if count is None or count <= cap:
            continue
        pins[path] = min(bound, count)
    data["pinned"] = dict(sorted(pins.items()))
    (repo_root / DATA_PATH).write_text(
        json.dumps(data, indent=2, sort_keys=False) + "\n", encoding="utf-8"
    )
    print(f"source size ratchet: {len(pins)} pins kept")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", choices=("check", "update"))
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    args = parser.parse_args(argv)
    if args.command == "check":
        return check(args.repo_root)
    return update(args.repo_root)


if __name__ == "__main__":
    sys.exit(main())
