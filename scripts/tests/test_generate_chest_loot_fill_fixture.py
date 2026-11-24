import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "scripts/generate-chest-loot-fill-fixture.py"
SPEC = importlib.util.spec_from_file_location("chest_loot_fill_fixture", PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class ChestLootFillFixtureTests(unittest.TestCase):
    def valid_report(self) -> str:
        header = "\t".join(
            [
                "H",
                "version=26.1",
                f"tables={MODULE.TABLE_COUNT}",
                "seeds=" + ",".join(str(seed) for seed in MODULE.SEEDS),
                "origin=" + ",".join(repr(value) for value in MODULE.ORIGIN),
                f"slots={MODULE.SLOTS}",
            ]
        )
        rows = [header]
        for index in range(MODULE.TABLE_COUNT):
            table = f"minecraft:chests/t{index}"
            for seed in MODULE.SEEDS:
                marker = MODULE.REACHED[index % len(MODULE.REACHED)]
                stack = "{components:{" + marker + "1},count:1,id:\"minecraft:stone\"}"
                rows.append(f"F\t{table}\t{seed}\t0\t{stack}")
                rows.append(f"E\t{table}\t{seed}\t1")
        return "\n".join(rows) + "\n"

    def test_parser_accepts_every_table_under_every_seed(self) -> None:
        artifact = MODULE.parse_report(self.valid_report())
        self.assertEqual(len(artifact["tables"]), MODULE.TABLE_COUNT)

    def test_parser_rejects_a_short_fill_and_a_missing_seed(self) -> None:
        report = self.valid_report()
        seed = MODULE.SEEDS[0]
        short = report.replace(f"E\tminecraft:chests/t0\t{seed}\t1\n", f"E\tminecraft:chests/t0\t{seed}\t2\n", 1)
        self.assertRaises(ValueError, MODULE.parse_report, short)
        lines = report.splitlines()
        self.assertRaises(ValueError, MODULE.parse_report, "\n".join(lines[:-2]) + "\n")


if __name__ == "__main__":
    unittest.main()
