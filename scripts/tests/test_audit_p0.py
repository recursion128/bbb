"""Unit tests for cross-platform P0 allowlist classification audit."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def _load_audit_p0():
    spec = importlib.util.spec_from_file_location(
        "audit_p0", REPO_ROOT / "scripts" / "audit_p0.py"
    )
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


audit_p0 = _load_audit_p0()


class AuditP0Tests(unittest.TestCase):
    def test_real_workspace_passes_allowlist_classification(self) -> None:
        self.assertEqual(audit_p0.run_audit(REPO_ROOT), 0)

    def test_unclassified_hit_fails_with_location(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            src = root / "crates" / "demo" / "src"
            src.mkdir(parents=True)
            (src / "lib.rs").write_text(
                "// allowed fallback\n// unallowed residual hit\n",
                encoding="utf-8",
            )
            allowlist = root / "allowlist.txt"
            allowlist.write_text(
                "^crates/demo/src/lib\\.rs$\tallowed[[:space:]]+fallback\n",
                encoding="utf-8",
            )
            self.assertEqual(audit_p0.run_audit(root, allowlist), 1)


if __name__ == "__main__":
    unittest.main()
