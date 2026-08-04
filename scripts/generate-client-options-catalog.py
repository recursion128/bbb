#!/usr/bin/env python3
"""Generate the Vanilla 26.1 ``options.txt`` key catalog."""

from __future__ import annotations

import argparse
import difflib
import json
from dataclasses import replace
from pathlib import Path
import sys

from client_options_catalog_extractor import (
    ClientOptionsCatalogExtractionError,
    client_option_entries,
    resolve_enum_default,
)
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = REPO_ROOT / "docs/evidence/vanilla-26.1/client-options-catalog.json"
OPTIONS_SOURCE = "net/minecraft/client/Options.java"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def _enum_source_loader(source_root: Path, package: str):
    """Resolve the file that declares an enum named in `Options`.

    A nested enum (`MusicManager.MusicFrequency`) lives in its imported owner's
    file, an imported enum in its own, and everything else in `Options`' own
    package.
    """

    cache: dict[str, object] = {}

    def load(qualifier: str, imports: dict[str, str]):
        if qualifier in cache:
            return cache[qualifier]
        outer = qualifier.split(".")[0]
        qualified = imports.get(outer)
        if qualified is None:
            qualified = f"{package}.{outer}"
        relative = qualified.replace(".", "/") + ".java"
        path = source_root / relative
        if not path.is_file():
            raise ClientOptionsCatalogExtractionError(
                f"cannot locate the source that declares {qualifier}"
            )
        parsed = parse_source(relative, path.read_bytes())
        cache[qualifier] = parsed
        return parsed

    return load


def render_catalog(source_root: Path) -> str:
    raw = (source_root / OPTIONS_SOURCE).read_bytes()
    source = parse_source(OPTIONS_SOURCE, raw)
    options, groups = client_option_entries(source)
    load_enum = _enum_source_loader(source_root, source.package)
    options = [
        option
        if not option.encoding.startswith("enum")
        else replace(
            option,
            default=resolve_enum_default(
                load_enum, source, option.encoding, option.default
            ),
        )
        for option in options
    ]
    document = {
        "generated_by": "scripts/generate-client-options-catalog.py",
        "vanilla_version": "26.1",
        "source_file": OPTIONS_SOURCE,
        "line_format": "name:value, split on the first ':' only; bad lines are skipped",
        "options": [
            {
                "order": option.order,
                "key": option.key,
                "field": option.field,
                "kind": option.kind,
                "encoding": option.encoding,
                "default": option.default,
            }
            for option in options
        ],
        "dynamic_groups": [
            {"order": group.order, "prefix": group.prefix, "kind": group.kind}
            for group in groups
        ],
    }
    return json.dumps(document, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    args = parse_args()
    rendered = render_catalog(args.source_root)
    if args.check:
        current = args.output.read_text(encoding="utf-8") if args.output.exists() else ""
        if current == rendered:
            print("client options catalog: checked output is current")
            return 0
        sys.stdout.writelines(
            difflib.unified_diff(
                current.splitlines(keepends=True),
                rendered.splitlines(keepends=True),
                fromfile=str(args.output),
                tofile="generated",
            )
        )
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    print(f"client options catalog: wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
