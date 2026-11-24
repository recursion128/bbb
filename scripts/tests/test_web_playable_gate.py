"""Regression tests for Web playable builds and performance receipts."""

from __future__ import annotations

import gzip
import hashlib
import http.server
import importlib.util
import os
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from typing import Any
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parents[2]


def _load_web_playable_gate() -> Any:
    spec = importlib.util.spec_from_file_location(
        "web_playable_gate", REPO_ROOT / "scripts" / "web_playable_gate.py"
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


web_gate = _load_web_playable_gate()


class WebPlayableGatePerfTests(unittest.TestCase):
    def test_existing_bundle_only_skips_build_with_explicit_opt_in(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            wasm_dir = Path(tmp) / "wasm-dist"
            pkg_dir = wasm_dir / "pkg"
            pkg_dir.mkdir(parents=True)
            (pkg_dir / "bbb_client_bg.wasm").touch()
            (pkg_dir / "bbb_client.js").touch()

            target_dir = Path(tmp) / "target"
            debug_dir = target_dir / "debug"
            debug_dir.mkdir(parents=True)
            suffix = ".exe" if sys.platform == "win32" else ""
            for name in ("bbb-server", "bbb-web-host"):
                (debug_dir / f"{name}{suffix}").touch()

            env = {"CARGO_TARGET_DIR": str(target_dir), "BBB_XTASK_BIN": "xtask"}
            with patch.dict(os.environ, env), patch.object(web_gate.subprocess, "run") as run:
                web_gate.ensure_builds(wasm_dir, skip_build=False)
                commands = [call.args[0][1] for call in run.call_args_list]
                self.assertEqual(commands, ["build-web", "build"])

                run.reset_mock()
                web_gate.ensure_builds(wasm_dir, skip_build=True)
                run.assert_not_called()

    def test_frame_time_stats_computes_percentiles_and_filters_invalid_samples(self) -> None:
        empty = web_gate.compute_frame_time_stats([])
        self.assertEqual(empty["sample_count"], 0)
        self.assertIsNone(empty["p50_ms"])
        self.assertIsNone(empty["p95_ms"])
        self.assertIsNone(empty["p99_ms"])

        samples = [float(i) for i in range(1, 102)] + [-1.0, float("nan"), float("inf")]
        stats = web_gate.compute_frame_time_stats(samples)
        self.assertEqual(stats["sample_count"], 101)
        self.assertEqual(stats["min_ms"], 1.0)
        self.assertEqual(stats["p50_ms"], 51.0)
        self.assertEqual(stats["p95_ms"], 96.0)
        self.assertEqual(stats["p99_ms"], 100.0)
        self.assertEqual(stats["max_ms"], 101.0)

    def test_probe_served_wasm_and_build_perf_telemetry(self) -> None:
        raw_wasm = b"\x00asm\x01\x00\x00\x00" + (b"x" * 4096)
        gz_wasm = gzip.compress(raw_wasm)
        core_zip = b"PK\x03\x04" + (b"z" * 2048)

        class StubGatewayHandler(http.server.BaseHTTPRequestHandler):
            def do_GET(self) -> None:  # noqa: N802
                if self.path == "/pkg/bbb_client_bg.wasm":
                    self.send_response(200)
                    self.send_header("Content-Type", "application/wasm")
                    self.send_header("Content-Encoding", "gzip")
                    self.send_header("Content-Length", str(len(gz_wasm)))
                    self.end_headers()
                    self.wfile.write(gz_wasm)
                    return
                if self.path == "/assets/core-26.1.zip":
                    self.send_response(200)
                    self.send_header("Content-Type", "application/zip")
                    self.send_header("Content-Length", str(len(core_zip)))
                    self.end_headers()
                    self.wfile.write(core_zip)
                    return
                self.send_response(404)
                self.end_headers()

            def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
                pass

        httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), StubGatewayHandler)
        port = httpd.server_address[1]
        thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        thread.start()
        try:
            wasm_sha256, payloads = web_gate.probe_gateway_payloads(port)
        finally:
            httpd.shutdown()
            httpd.server_close()

        self.assertEqual(wasm_sha256, hashlib.sha256(raw_wasm).hexdigest())
        self.assertEqual(payloads["wasm_raw_bytes"], len(raw_wasm))
        self.assertEqual(payloads["wasm_gzip_bytes"], len(gz_wasm))
        self.assertEqual(payloads["core_bundle_bytes"], len(core_zip))

        perf = web_gate.build_perf_telemetry(
            {
                "frameTimesMs": [16.0, 16.5, 17.0, 22.0],
                "initialWasmBytes": 128 * 1024 * 1024,
                "peakWasmBytes": 256 * 1024 * 1024,
                "finalWasmBytes": 240 * 1024 * 1024,
            },
            payloads,
            first_playable_ms=1500.0,
            max_first_playable_ms=2000.0,
            max_wasm_mib=512.0,
            max_frame_p95_ms=33.3,
        )
        self.assertEqual(perf["first_playable_ms"], 1500.0)
        self.assertEqual(perf["wasm_memory"]["initial_mib"], 128.0)
        self.assertEqual(perf["wasm_memory"]["peak_mib"], 256.0)
        self.assertEqual(perf["wasm_memory"]["final_mib"], 240.0)
        self.assertEqual(perf["frame_times"]["sample_count"], 4)
        self.assertEqual(perf["budgets"]["max_first_playable_ms"], 2000.0)
        self.assertEqual(perf["budgets"]["max_wasm_mib"], 512.0)
        self.assertEqual(perf["budgets"]["max_frame_p95_ms"], 33.3)

    def test_cli_budget_flags_and_opt_in_enforcement(self) -> None:
        parser = web_gate.build_arg_parser(Path("/tmp"))
        defaults = parser.parse_args([])
        self.assertIsNone(defaults.max_first_playable_ms)
        self.assertIsNone(defaults.max_wasm_mib)
        self.assertIsNone(defaults.max_frame_p95_ms)

        parsed = parser.parse_args([
            "--max-first-playable-ms", "2000", "--max-wasm-mib", "384", "--max-frame-p95-ms", "25.5"
        ])
        self.assertEqual(parsed.max_first_playable_ms, 2000.0)
        self.assertEqual(parsed.max_wasm_mib, 384.0)
        self.assertEqual(parsed.max_frame_p95_ms, 25.5)

        for bad_arg in ("0", "-10", "nan", "inf"):
            with self.assertRaises(SystemExit):
                parser.parse_args(["--max-wasm-mib", bad_arg])

        slow_perf = web_gate.build_perf_telemetry(
            {
                "frameTimesMs": [45.0, 50.0, 55.0],
                "initialWasmBytes": 600 * 1024 * 1024,
                "peakWasmBytes": 640 * 1024 * 1024,
                "finalWasmBytes": 640 * 1024 * 1024,
            },
            {},
            first_playable_ms=1500.0,
        )
        web_gate.enforce_perf_budgets(slow_perf)
        for name, limit in (
            ("max_first_playable_ms", 1000.0),
            ("max_wasm_mib", 512.0),
            ("max_frame_p95_ms", 33.3),
        ):
            with self.subTest(budget=name), self.assertRaisesRegex(
                web_gate.WebGateError, name.replace("_", "-")
            ):
                budgets = {**slow_perf["budgets"], name: limit}
                web_gate.enforce_perf_budgets({**slow_perf, "budgets": budgets})

    def test_build_arg_parser_honors_target_root_and_scoped_target_name(self) -> None:
        with patch.dict(
            os.environ,
            {"BBB_TARGET_ROOT": "/custom-root"},
            clear=True,
        ):
            args = web_gate.build_arg_parser().parse_args([])
            self.assertEqual(args.wasm_dir, Path("/custom-root/bbb-web-dist"))
            self.assertEqual(args.cache_dir, Path("/custom-root/bbb-web-cache"))

        with patch.dict(
            os.environ,
            {
                "BBB_TARGET_ROOT": "/custom-root",
                "BBB_CARGO_TARGET_NAME": "swarm-web",
            },
            clear=True,
        ):
            args = web_gate.build_arg_parser().parse_args([])
            self.assertEqual(args.wasm_dir, Path("/custom-root/bbb-web-dist-swarm-web"))
            self.assertEqual(args.cache_dir, Path("/custom-root/bbb-web-cache-swarm-web"))

        with patch.dict(
            os.environ,
            {
                "BBB_TARGET_ROOT": "/custom-root",
                "BBB_CARGO_TARGET_NAME": "../bad",
            },
            clear=True,
        ):
            args = web_gate.build_arg_parser().parse_args([])
            self.assertEqual(args.wasm_dir, Path("/custom-root/bbb-web-dist"))
            self.assertEqual(args.cache_dir, Path("/custom-root/bbb-web-cache"))

        with patch.dict(
            os.environ,
            {
                "BBB_TARGET_ROOT": "/custom-root",
                "BBB_WEB_OUT_DIR": "/explicit-dist",
                "BBB_WEB_CACHE_DIR": "/explicit-cache",
            },
            clear=True,
        ):
            args = web_gate.build_arg_parser().parse_args([])
            self.assertEqual(args.wasm_dir, Path("/explicit-dist"))
            self.assertEqual(args.cache_dir, Path("/explicit-cache"))


if __name__ == "__main__":
    unittest.main()
