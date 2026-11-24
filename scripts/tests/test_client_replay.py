"""Capture plans and the MCP backend of the existing lockstep replay loop."""

from __future__ import annotations

import base64
import http.client
import json
import os
import socket
import tempfile
import threading
import unittest
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from unittest.mock import Mock, patch

import client_replay as replay


@contextmanager
def mcp_endpoint(tool_answer=None, *, status=200, rpc_error=None):
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            message = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append((self.path, dict(self.headers), message))
            method = message["method"]
            if method == "notifications/initialized":
                self.send_response(202)
                self.end_headers()
                return
            response = {"jsonrpc": "2.0", "id": message["id"]}
            if method == "initialize":
                response["result"] = {"protocolVersion": "2025-06-18"}
            elif rpc_error is not None:
                response["error"] = rpc_error
            else:
                response["result"] = tool_answer
            body = json.dumps(response).encode()
            self.send_response(200 if method == "initialize" else status)
            self.send_header("Content-Length", str(len(body)))
            if method == "initialize":
                self.send_header("Mcp-Session-Id", "replay-session")
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    client = object.__new__(replay.Client)
    client.transport = "mcp"
    client.mcp_session = None
    client.mcp_version = "2025-06-18"
    client.mcp_id = 0
    client.mcp_http = http.client.HTTPConnection(*server.server_address, timeout=2)
    client.last_content = []
    try:
        yield client, requests
    finally:
        client.mcp_http.close()
        server.shutdown()
        thread.join()
        server.server_close()


class WestonTests(unittest.TestCase):
    @unittest.skipUnless(hasattr(socket, "AF_UNIX"), "AF_UNIX socket required")
    def test_stale_socket_is_skipped_and_ready_requires_a_connection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stale = root / f"bbb-replay-{os.getpid()}-stale"
            fresh = root / f"bbb-replay-{os.getpid()}-fresh"
            old_server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            new_server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            old_server.bind(str(stale))
            process = Mock()
            process.poll.return_value = None
            sleeps = 0

            def advance(_seconds):
                nonlocal sleeps
                sleeps += 1
                if sleeps == 1:
                    new_server.bind(str(fresh))
                    Path(f"{fresh}.lock").touch()
                elif sleeps == 2:
                    new_server.listen()

            try:
                with patch.dict(os.environ, {"XDG_RUNTIME_DIR": directory}), patch(
                    "client_replay.uuid.uuid4", side_effect=[Mock(hex="stale"), Mock(hex="fresh")]
                ), patch("client_replay.subprocess.Popen", return_value=process), patch(
                    "client_replay.time.sleep", side_effect=advance
                ):
                    weston = replay.Weston(root)
                self.assertEqual(fresh.name, weston.socket)
                self.assertEqual(2, sleeps)
                weston.close()
                process.kill.assert_called_once()
                process.wait.assert_called_once()
                self.assertFalse(fresh.exists())
                self.assertFalse(Path(f"{fresh}.lock").exists())
                self.assertTrue(stale.exists())
            finally:
                old_server.close()
                new_server.close()

    def test_startup_timeout_stops_process_and_removes_its_lock(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / f"bbb-replay-{os.getpid()}-timeout"
            process = Mock()
            process.poll.return_value = None

            def launch(*_args, **_kwargs):
                Path(f"{path}.lock").touch()
                return process

            with patch.dict(os.environ, {"XDG_RUNTIME_DIR": directory}), patch(
                "client_replay.uuid.uuid4", return_value=Mock(hex="timeout")
            ), patch("client_replay.subprocess.Popen", side_effect=launch), patch(
                "client_replay.time.monotonic", side_effect=[0, 11]
            ):
                with self.assertRaisesRegex(replay.ReplayError, "weston did not start"):
                    replay.Weston(root)
            process.kill.assert_called_once()
            process.wait.assert_called_once()
            self.assertFalse(Path(f"{path}.lock").exists())


class McpBackendTests(unittest.TestCase):
    def test_one_session_drives_calls_using_structured_content(self):
        answer = {
            "structuredContent": {"quiescent": True},
            "content": [{"type": "text", "text": "not JSON and not a result"}],
            "isError": False,
        }
        with mcp_endpoint(answer) as (client, requests):
            self.assertEqual({"quiescent": True}, client.call("sim.lockstep_status"))
            client.call("input.key", {"key": "forward", "pressed": True})
        self.assertEqual(
            ["initialize", "notifications/initialized", "tools/call", "tools/call"],
            [message["method"] for _, _, message in requests],
        )
        for path, headers, _ in requests[1:]:
            self.assertEqual("/mcp", path)
            self.assertEqual("replay-session", headers["Mcp-Session-Id"])
            self.assertEqual("2025-06-18", headers["Mcp-Protocol-Version"])
        self.assertEqual(
            {"name": "input.key", "arguments": {"key": "forward", "pressed": True}},
            requests[-1][2]["params"],
        )

    def test_http_rpc_tool_errors_and_text_only_results_fail(self):
        for kwargs in (
            {"status": 503},
            {"rpc_error": {"code": -32601, "message": "unknown"}},
            {"tool_answer": {"isError": True}},
            {"tool_answer": {"content": [{"type": "text", "text": "{}"}]}},
        ):
            with self.subTest(kwargs=kwargs), mcp_endpoint(**kwargs) as (client, _):
                with self.assertRaises(replay.ReplayError):
                    client.call("sim.step", {"ticks": 1})

    def test_mcp_startup_selects_only_mcp_listener(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            tools = replay.Tools(out, "mcp")
            weston = Mock(runtime=str(out), socket="test-wayland")
            with patch.object(replay.Tools, "bin", return_value=out / "bbb-client"), patch(
                "client_replay.subprocess.Popen"
            ) as popen:
                client = replay.Client(tools, out, weston, 25565, replay.Scenario("test", 1))
            arguments = popen.call_args.args[0]
            self.assertIn("--mcp-bind", arguments)
            self.assertIn("--lockstep", arguments)
            self.assertNotIn("--control", arguments)
            client.mcp_http.close()

    def test_screenshot_requires_completed_receipt_and_matching_returned_image(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "shot.png"
            data = base64.b64decode(
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMC"
                "AO+aZ9sAAAAASUVORK5CYII="
            )
            receipt = {
                "completed": True, "path": str(path), "width": 1, "height": 1,
                "mime_type": "image/png",
            }
            image = {
                "type": "image", "mimeType": "image/png", "data": base64.b64encode(data).decode(),
            }
            with mcp_endpoint({"structuredContent": receipt, "content": [image]}) as (client, _):
                session = replay.Session(replay.Scenario("test", 1), client, Mock(), path.parent)
                session.barrier = Mock()

                def screenshot(method, params):
                    path.write_bytes(data)
                    return replay.Client.call(client, method, params)

                client.call = screenshot
                session.screenshot(path, 0)
                session.barrier.assert_called_once_with(0)
                path.write_bytes(b"stale image")
                with self.assertRaisesRegex(replay.ReplayError, "does not match"):
                    session.completed_screenshot(path, receipt)
                with self.assertRaisesRegex(replay.ReplayError, "completed receipt"):
                    session.completed_screenshot(path, {"queued": True})


class CapturePlanTests(unittest.TestCase):
    def test_plan_restores_ordered_inputs_and_masks_without_injecting_grab(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scenario = replay.Scenario("saved", 150, world=Path("record-only"), grab_at=2)
            scenario.control(3, "input.text", text="hello").key(3, "jump")
            scenario.shot(6, "view", masks=[(0, 1, 2, 3)])
            replay.write_plan(root, scenario)
            saved = replay.read_plan(root, "saved")
            self.assertEqual(replay.scenario_plan(scenario), replay.scenario_plan(saved))
            self.assertIsNone(saved.world)
            self.assertNotIn(100, saved.actions)
            self.assertEqual(["input.text", "input.key"], [v[0] for _, v in saved.actions[3]])
            self.assertNotIn("grab_at", json.loads((root / "scenario.json").read_text()))

    def test_explicit_capture_survives_removed_factory_and_bad_plan_never_falls_back(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(replay, "CACHE", Path(directory)),
        ):
            capture = replay.CACHE / "removed" / "capture"
            capture.mkdir(parents=True)
            scenario = replay.Scenario("removed", 120)
            replay.write_plan(capture, scenario)
            self.assertEqual("removed", replay.selected(["removed"], captures=True)[0].name)
            self.assertIsNone(replay.read_plan(capture.parent, "removed"))
            plan = replay.scenario_plan(scenario)
            for invalid in (
                "{broken", json.dumps({**plan, "version": 2}), json.dumps({**plan, "actions": {}}),
            ):
                (capture / "scenario.json").write_text(invalid)
                with self.assertRaises(replay.ReplayError):
                    replay.selected(["removed"], captures=True)

    def test_replay_uses_capture_plan_and_writes_actual_plan_to_output(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(replay, "CACHE", Path(directory)),
        ):
            capture = replay.CACHE / "saved" / "capture"
            capture.mkdir(parents=True)
            saved = replay.Scenario("saved", 2, grab_at=0).control(1, "input.text", text="saved")
            replay.write_plan(capture, saved)
            (capture / "session.json").write_text('{"compression": false}')
            (capture / "clientbound.bin").write_bytes(b"")
            out = Path(directory) / "out"
            with patch.object(replay, "Weston"), patch.object(replay, "Client"), patch.object(
                replay, "Link"
            ) as link, patch.object(replay.Session, "run", autospec=True) as run, patch.object(
                replay, "cpu_seconds", return_value=0
            ), patch.object(replay, "peak_rss", return_value=0):
                link.return_value.serverbound = []
                replay.replay(replay.Tools(Path(directory)), replay.Scenario("saved", 9), out)
            self.assertEqual(2, run.call_args.args[0].scenario.steps)
            self.assertEqual(
                replay.scenario_plan(saved), replay.scenario_plan(replay.read_plan(out, "saved")),
            )

    def test_compare_reads_output_masks_and_rejects_different_plans(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            left, right = root / "left", root / "right"
            actual = replay.Scenario("saved", 120).shot(110, "view", masks=[(1, 2, 3, 4)])
            factory = replay.Scenario("saved", 120).shot(110, "view", masks=[(9, 8, 7, 6)])
            for output, data in ((left, b"left"), (right, b"right")):
                (output / "shots").mkdir(parents=True)
                (output / "shots" / "view.png").write_bytes(data)
                (output / "files.json").write_text("{}")
                replay.write_plan(output, actual)
            with patch.object(replay.Tools, "bin", return_value=Path("diff")), patch(
                "client_replay.subprocess.run", return_value=Mock(stdout="")
            ) as diff:
                replay.compare_scenario(replay.Tools(root), factory, left, right, root / "diff")
            self.assertEqual([[1, 2, 3, 4]], json.loads(diff.call_args.kwargs["input"])[0]["masks"])
            actual.control(115, "input.text", text="changed plan")
            replay.write_plan(right, actual)
            report = replay.compare_scenario(
                replay.Tools(root), factory, left, right, root / "diff"
            )
            self.assertIsNotNone(report["plan"])
            self.assertFalse(replay.passes(report, 32, 1_000_000))

    def test_old_outputs_without_plans_keep_factory_masks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            left, right = root / "left", root / "right"
            factory = replay.Scenario("saved", 120).shot(110, "view", masks=[(1, 2, 3, 4)])
            for output in (left, right):
                output.mkdir()
                (output / "files.json").write_text("{}")
            replay.write_plan(right, factory)
            report = replay.compare_scenario(
                replay.Tools(root), factory, left, right, root / "diff"
            )
            self.assertTrue(replay.passes(report, 0, 0))

    def test_summary_includes_frame_time_percentiles_and_mesh_worker_counters(self):
        stats = replay.compute_frame_time_stats([10.0, 20.0, 30.0, 40.0, 100.0])
        self.assertEqual(5, stats["sample_count"])
        self.assertEqual(30.0, stats["p50_ms"])
        self.assertGreater(stats["p95_ms"], stats["p50_ms"])
        self.assertGreaterEqual(stats["p99_ms"], stats["p95_ms"])

        mesh = replay.summarize_mesh_worker(
            {
                "terrain_mesh_recent_build_nanos": 450_000,
                "terrain_mesh_peak_build_nanos": 900_000,
                "terrain_mesh_queue_full": 2,
                "terrain_mesh_jobs_completed": 8,
            }
        )
        self.assertEqual(450_000, mesh["build_nanos"])
        self.assertEqual(900_000, mesh["peak_build_nanos"])
        self.assertEqual(2, mesh["queue_full"])
        self.assertEqual(8, mesh["jobs_completed"])

        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "out"
            out.mkdir()
            scenario = replay.Scenario("summary", 2)

            class FakeClient:
                transport = "control"

                def __init__(self):
                    self.frames = 0

                def check_alive(self):
                    pass

                def call(self, method, _params=None):
                    if method == "sim.lockstep_status":
                        return {
                            "quiescent": True,
                            "net_frames_read": 0,
                            "net_frames_written": 0,
                            "frames_rendered": self.frames,
                        }
                    if method == "sim.step":
                        self.frames += 1
                        return {}
                    if method == "renderer.counters":
                        return {
                            "terrain_mesh_recent_build_nanos": 123_456,
                            "terrain_mesh_peak_build_nanos": 234_567,
                            "terrain_mesh_queue_full": 1,
                        }
                    return {}

            link = Mock()
            link.serverbound_count.return_value = 0
            session = replay.Session(scenario, FakeClient(), link, out)
            session.run(lambda _w: [], lambda _c: None)
            self.assertEqual(2, len(session.frame_times_ms))
            self.assertEqual(123_456, session.mesh_counters["terrain_mesh_recent_build_nanos"])
            self.assertEqual(1, session.mesh_counters["terrain_mesh_queue_full"])


if __name__ == "__main__":
    unittest.main()
