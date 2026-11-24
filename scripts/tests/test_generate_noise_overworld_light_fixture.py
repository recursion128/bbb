import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "scripts/generate-noise-overworld-light-fixture.py"
SPEC = importlib.util.spec_from_file_location("overworld_light_fixture", PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class OverworldLightFixtureTests(unittest.TestCase):
    @staticmethod
    def bits(*indices: int) -> str:
        return "".join("1" if index in indices else "0" for index in range(26))

    @staticmethod
    def sha(value: int) -> str:
        return f"{value:064x}"

    def valid_report(self) -> str:
        rows = [
            "\t".join(
                [
                    "P",
                    self.bits(1, 3),
                    self.bits(2),
                    self.bits(0),
                    self.bits(4),
                    "2",
                    "1",
                ]
            ),
            f"L\tS\t1\t{self.sha(1)}",
            f"L\tS\t3\t{self.sha(2)}",
            f"L\tB\t2\t{self.sha(3)}",
        ]
        digest = MODULE._digest_rows(rows)
        return "\n".join([MODULE.HEADER, *rows, f"A\t{digest}\t{digest}", ""])

    def test_parser_accepts_exact_masks_arrays_and_order_invariance(self) -> None:
        artifact = MODULE.parse_report(self.valid_report())
        projection = artifact["projection"]
        self.assertEqual(projection["sky_mask"], self.bits(1, 3))
        self.assertEqual(
            [row["section_y"] for row in projection["sky_updates"]],
            [-4, -2],
        )
        self.assertEqual(artifact["input"]["base_columns"], "all-air")
        self.assertEqual(artifact["invariance"]["init_orders"], ["forward", "reverse"])
        self.assertIn("natural-features-input", artifact["scope"]["does_not_certify"])
        self.assertIn("region-save-resume", artifact["scope"]["does_not_certify"])

    def test_parser_rejects_mask_overlap_and_cardinality_drift(self) -> None:
        report = self.valid_report()
        self.assertRaises(
            ValueError,
            MODULE.parse_report,
            report.replace(self.bits(0), self.bits(1), 1),
        )
        self.assertRaises(
            ValueError,
            MODULE.parse_report,
            report.replace("\t2\t1\n", "\t3\t1\n", 1),
        )

    def test_parser_rejects_layer_order_or_digest_drift(self) -> None:
        report = self.valid_report()
        self.assertRaises(
            ValueError,
            MODULE.parse_report,
            report.replace("L\tS\t1", "L\tS\t3", 1),
        )
        self.assertRaises(
            ValueError,
            MODULE.parse_report,
            report.replace("L\tS\t3", "L\tB\t3", 1),
        )
        self.assertRaises(
            ValueError,
            MODULE.parse_report,
            report.rsplit("\t", 1)[0] + "\t" + self.sha(99) + "\n",
        )

    def test_parser_rejects_scope_header_drift(self) -> None:
        self.assertRaises(
            ValueError,
            MODULE.parse_report,
            self.valid_report().replace("status_claim=component-only", "status_claim=light", 1),
        )


if __name__ == "__main__":
    unittest.main()
