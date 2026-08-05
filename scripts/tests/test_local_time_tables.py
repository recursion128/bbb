from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import local_time_tables  # noqa: E402


class LocalTimeTablesTests(unittest.TestCase):
    def test_checked_in_module_matches_tsv_inputs(self) -> None:
        generated = local_time_tables.generate()
        current = local_time_tables.OUTPUT.read_text(encoding="utf-8")
        self.assertEqual(
            current,
            generated,
            "tables_26_1.rs is stale; run scripts/local_time_tables.py --write",
        )

    def test_generated_module_covers_every_table(self) -> None:
        generated = local_time_tables.generate()
        for table in (
            "WEEK_DATA",
            "QUARTERS",
            "GMT_ZERO",
            "UNKNOWN_CITY",
            "ROOT_ZONE_LOCATION",
        ):
            self.assertIn(f"pub(super) const {table}:", generated)


if __name__ == "__main__":
    unittest.main()
