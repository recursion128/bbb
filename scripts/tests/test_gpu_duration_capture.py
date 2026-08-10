from __future__ import annotations

import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import gpu_duration_capture as capture  # noqa: E402

FIXTURE = SCRIPTS / "fixtures" / "gpu-duration" / "perf-script-sample.txt"

# The three pids the checked-in capture excerpt carries.
BBB_VULKAN_PID = 3332795
WESTON_PID = 3339885
OFFICIAL_CLIENT_PID = 3340528


class GpuDurationCaptureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        with FIXTURE.open(encoding="utf-8") as handle:
            cls.indexed = capture.parse_perf_script(handle)
        cls.jobs = capture.join_jobs(cls.indexed)

    def test_every_client_submission_reaches_a_complete_chain(self) -> None:
        self.assertEqual(capture.association_ratio(self.indexed, self.jobs), 1.0)
        self.assertEqual(len(self.jobs), len(self.indexed["submits"]))

    def test_both_graphics_apis_are_measured_by_the_same_chain(self) -> None:
        # radeonsi/OpenGL (the official client) and RADV/Vulkan (bbb) submit
        # through the same amdgpu path, which is what makes one capture
        # API-neutral rather than two instrumented clients.
        pids = {job.pid for job in self.jobs}
        self.assertIn(OFFICIAL_CLIENT_PID, pids)
        self.assertIn(BBB_VULKAN_PID, pids)

    def test_durations_follow_the_submit_dispatch_finish_ordering(self) -> None:
        for job in self.jobs:
            self.assertLessEqual(job.submit_ns, job.run_ns)
            self.assertLessEqual(job.run_ns, job.finished_ns)
            self.assertEqual(job.queue_wait_ns, job.run_ns - job.submit_ns)
            self.assertEqual(job.execution_ns, job.finished_ns - job.run_ns)
            self.assertGreater(job.execution_ns, 0)

    def test_compositor_submissions_are_excluded_from_client_numbers(self) -> None:
        included = capture.summarize(self.jobs)
        self.assertIn(WESTON_PID, {client["pid"] for client in included["clients"]})
        excluded = capture.summarize(self.jobs, excluded_pids=(WESTON_PID,))
        self.assertNotIn(WESTON_PID, {client["pid"] for client in excluded["clients"]})
        self.assertEqual(excluded["excluded_pids"], [WESTON_PID])
        self.assertLess(excluded["job_count"], included["job_count"])

    def test_client_selection_reports_only_the_measured_client(self) -> None:
        report = capture.summarize(self.jobs, client_pids=(OFFICIAL_CLIENT_PID,))
        self.assertEqual([client["pid"] for client in report["clients"]], [OFFICIAL_CLIENT_PID])
        stats = report["clients"][0]["gpu_execution"]
        self.assertEqual(stats["count"], report["job_count"])
        self.assertGreater(stats["p50_us"], 0.0)
        self.assertGreaterEqual(stats["max_us"], stats["p99_us"])
        self.assertGreaterEqual(stats["p99_us"], stats["p50_us"])

    def test_scheduled_fence_context_is_not_mistaken_for_completion(self) -> None:
        # drm/scheduler allocates two fence contexts per entity: `scheduled` at
        # `fence_context` and `finished` at `+1`. Joining on the wrong one would
        # silently report the dispatch latency as the execution duration.
        for job in self.jobs:
            scheduled = self.indexed["signals"].get(
                (capture.SCHEDULER_FENCE_DRIVER, job.context - 1, job.seqno)
            )
            if scheduled is None:
                continue
            self.assertLessEqual(scheduled, job.finished_ns)

    def test_partial_chains_are_dropped_rather_than_truncated(self) -> None:
        # A capture window that closes before a job finishes must contribute
        # nothing instead of a short duration.
        indexed = {
            "submits": {(7, 1): {"timestamp_ns": 10, "pid": 5, "comm": "c", "ring": "gfx_0.0.0"}},
            "runs": {(7, 1): {"timestamp_ns": 20, "pid": 9, "comm": "k", "ring": "gfx_0.0.0"}},
            "signals": {},
        }
        self.assertEqual(capture.join_jobs(indexed), [])
        self.assertEqual(capture.association_ratio(indexed, []), 0.0)

    def test_non_graphics_rings_are_separated_from_the_headline_metric(self) -> None:
        graphics = capture.summarize(self.jobs)
        self.assertTrue(
            all(
                ring.startswith(capture.GRAPHICS_RING_PREFIX)
                for client in graphics["clients"]
                for ring in client["rings"]
            )
        )
        self.assertTrue(graphics["graphics_only"])

    def test_summarize_script_counts_kernel_internal_jobs_separately(self) -> None:
        report = capture.summarize_script(FIXTURE)
        self.assertEqual(report["association_ratio"], 1.0)
        self.assertEqual(report["kernel_internal_jobs"], 0)
        self.assertEqual(report["submitted_jobs"], report["associated_jobs"])

    def test_linux_7_fence_trace_format_reaches_a_complete_chain(self) -> None:
        lines = [
            " client:cs0 4242 10.000000001: amdgpu:amdgpu_cs_ioctl: "
            "timeline=gfx_0.0.0, fence=91:7, ring_name=gfx_0.0.0, num_ibs=1\n",
            " kworker 99 10.000000002: amdgpu:amdgpu_sched_run_job: "
            "timeline=gfx_0.0.0, fence=91:7, ring_name=gfx_0.0.0, num_ibs=1\n",
            " swapper 0 10.000000003: dma_fence:dma_fence_signaled: "
            "driver=drm_sched timeline=gfx_0.0.0 context=91 seqno=7\n",
        ]
        indexed = capture.parse_perf_script(lines)
        jobs = capture.join_jobs(indexed)

        self.assertEqual(capture.association_ratio(indexed, jobs), 1.0)
        self.assertEqual(len(jobs), 1)
        self.assertEqual(jobs[0].pid, 4242)
        self.assertEqual(jobs[0].execution_ns, 1)

    def test_preflight_names_the_provisioning_for_each_blocker(self) -> None:
        report = capture.preflight(Path("/nonexistent-trace-root"))
        codes = {blocker["code"] for blocker in report["blockers"]}
        self.assertIn("tracepoint_ids_unreadable", codes)
        self.assertIn("trace_metadata_unreadable", codes)
        self.assertEqual(report["trace_metadata"], {})
        self.assertFalse(report["unprivileged_capture_ready"])
        for blocker in report["blockers"]:
            self.assertTrue(blocker["provision"].startswith("sudo "))

    def test_capture_cli_keeps_subcommand_separate_from_wrapped_command(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "capture"
            expected = {
                "command_returncode": 0,
                "script_returncode": 0,
                "perf_data": str(output / "gpu-duration.perf.data"),
                "script": str(output / "gpu-duration.script.txt"),
            }
            stdout = io.StringIO()
            with mock.patch.object(capture, "capture", return_value=expected) as run:
                with mock.patch.object(capture.sys, "platform", "linux"), mock.patch(
                    "sys.stdout", stdout
                ):
                    exit_code = capture.main(
                        ["capture", "--output", str(output), "--", "true"]
                    )

        self.assertEqual(exit_code, 0)
        run.assert_called_once_with(output, ["true"])
        self.assertEqual(json.loads(stdout.getvalue()), expected)


if __name__ == "__main__":
    unittest.main()
