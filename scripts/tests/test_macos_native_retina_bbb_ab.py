from __future__ import annotations

from pathlib import Path
import sys
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"
sys.path.insert(0, str(SCRIPTS))

import macos_native_retina as retina  # noqa: E402
import macos_native_retina_bbb_ab as bbb_ab  # noqa: E402


def counters(*, frames: int, draws: int, resident_bytes: int) -> dict[str, object]:
    value: dict[str, object] = {
        field: 0 for field in bbb_ab.WORKLOAD_COUNTER_FIELDS
    }
    value.update({field: 0 for field in bbb_ab.RESIDENT_COUNTER_FIELDS})
    value.update(
        {
            "frame_index": frames,
            "draw_calls": draws,
            "uploaded_sections": 200,
            "visible_sections": 150,
            "resident_bytes": resident_bytes,
            "terrain_selected_chunks": 144,
            "terrain_resident_chunks": 144,
            "terrain_selected_sections": 200,
            "terrain_resident_sections": 200,
            "terrain_cache_ready": True,
            "terrain_vertices": 20_000,
            "terrain_indices": 30_000,
            "opaque_faces": 2_000,
            "cutout_faces": 100,
            "translucent_faces": 50,
        }
    )
    return value


def run(*, frames: int, draws: int, resident_bytes: int) -> dict[str, object]:
    return {
        "profiles": [
            {
                "segment": segment,
                "renderer_counters_before": counters(
                    frames=0, draws=0, resident_bytes=resident_bytes
                ),
                "renderer_counters": counters(
                    frames=frames, draws=draws, resident_bytes=resident_bytes
                ),
                "process_memory": {
                    "resident_bytes": resident_bytes,
                    "physical_footprint_bytes": resident_bytes,
                },
            }
            for segment in retina.CAPTURE_SEGMENTS
        ]
    }


class DependencyAbTests(unittest.TestCase):
    def test_scope_set_matches_the_validator_exactly(self) -> None:
        observed = {(scope_type, scope_id) for scope_type, scope_id, _, _ in bbb_ab._scope_specs()}

        self.assertEqual(observed, retina._candidate_required_scopes())

    def test_workload_parity_uses_draws_per_frame_and_exact_residency(self) -> None:
        baseline = run(frames=100, draws=1_000, resident_bytes=50_000)
        candidate = run(frames=200, draws=2_000, resident_bytes=50_000)

        matched, signatures, failures = bbb_ab._workload_parity(
            [baseline, candidate]
        )

        self.assertTrue(matched)
        self.assertEqual(failures, [])
        self.assertEqual(
            signatures[0][retina.CAPTURE_SEGMENTS[0]]["draw_deltas"]["draw_calls"][
                "per_frame"
            ],
            10.0,
        )

    def test_workload_parity_rejects_resident_mismatch_before_performance(self) -> None:
        baseline = run(frames=100, draws=1_000, resident_bytes=50_000)
        candidate = run(frames=100, draws=1_000, resident_bytes=60_000)

        matched, _, failures = bbb_ab._workload_parity([baseline, candidate])

        self.assertFalse(matched)
        self.assertTrue(
            any(failure["field"] == "resident_end.resident_bytes" for failure in failures)
        )

    def test_memory_summary_uses_the_maximum_value_in_each_scope(self) -> None:
        runs = [run(frames=100, draws=1_000, resident_bytes=value) for value in range(100, 108)]

        summary = bbb_ab._memory_summary(
            runs, tuple(range(1, 9)), "baseline", None
        )

        self.assertEqual(summary["resident_bytes"], 107)
        self.assertEqual(summary["physical_footprint_bytes"], 107)

    def test_dependency_pair_cli_requires_a_subcommand(self) -> None:
        with self.assertRaises(SystemExit):
            bbb_ab.parse_args([])


if __name__ == "__main__":
    unittest.main()
