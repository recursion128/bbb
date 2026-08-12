#!/usr/bin/env python3
"""Regenerate or verify the direct Cargo dependency audit receipt."""

from __future__ import annotations

import argparse
from pathlib import Path

from direct_dependency_receipt import render_receipt


REPO_ROOT = Path(__file__).resolve().parent.parent
OUTPUT = REPO_ROOT / "docs/evidence/dependency-receipts/direct-workspace-dependencies.json"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = render_receipt(REPO_ROOT)
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            raise SystemExit(f"stale direct dependency receipt: {OUTPUT}")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
