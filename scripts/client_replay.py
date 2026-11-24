#!/usr/bin/env python3
"""Lockstep record/replay regression for bbb-client.

The client runs with `--lockstep`: its clock only moves on control `sim.step`.
Every scenario is a sequence of windows; each window releases the clientbound
frames that belong to it, waits until the client is quiescent, runs the
scenario's inputs and checkpoints, then steps one tick.

record   bbb-server <-> holding proxy <-> client. Server frames are held and
         released only at window starts, so a capture says exactly which
         frames the client had seen before every tick.
replay   A fake server releases a capture's frames at the same windows. The
         output is the serverbound packet stream, screenshots, the submitted
         audio commands and the final game directory.
compare  Two replay outputs: exact packets/audio/files, per-pixel screenshots.

Captures live in ~/.cache/bbb/client-replay/<scenario>/capture; record them
once on the refactor base and replay them on every candidate.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import http.client
import json
import math
import os
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
import uuid
import zlib
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bbb_tools.host.process_metrics import sample_process_metrics  # noqa: E402
from bbb_tools.python_runner import default_target_dir as shared_default_target_dir  # noqa: E402
from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402
from server_release_gate import Paths, ServerProcess, free_port  # noqa: E402


def default_cache_base() -> Path:
    if sys.platform == "win32" and os.environ.get("LOCALAPPDATA"):
        return Path(os.environ["LOCALAPPDATA"]) / "bbb"
    if sys.platform == "darwin":
        return Path.home() / "Library/Caches/bbb"
    if os.environ.get("XDG_CACHE_HOME"):
        return Path(os.environ["XDG_CACHE_HOME"]) / "bbb"
    return Path.home() / ".cache/bbb"


def default_vanilla_root() -> Path:
    return VanillaWorkspace.resolve("26.1").root


def default_target_debug_dir() -> Path:
    return shared_default_target_dir() / "debug"


CACHE = (
    Path(os.environ["BBB_CLIENT_REPLAY_CACHE"])
    if os.environ.get("BBB_CLIENT_REPLAY_CACHE")
    else default_cache_base() / "client-replay"
)
VANILLA_ROOT = default_vanilla_root()
TERRAIN_TEMPLATE = default_cache_base() / "bench/template"
WIDTH, HEIGHT = 854, 480
USERNAME = "bbb-replay"
BARRIER_TIMEOUT = 180.0
POLL = 0.004

# Phase transitions seen on the serverbound stream (packet_catalog.rs).
SB_LOGIN_ACKNOWLEDGED = 3
SB_FINISH_CONFIGURATION = 3
SB_CONFIGURATION_ACKNOWLEDGED = 16
CB_LOGIN_COMPRESSION = 3


class ReplayError(RuntimeError):
    pass


# --- framing -----------------------------------------------------------------


def read_varint(data: bytes, pos: int) -> tuple[int, int] | None:
    value = shift = 0
    while pos < len(data):
        byte = data[pos]
        pos += 1
        value |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return value, pos
        shift += 7
        if shift > 35:
            raise ReplayError("varint too long")
    return None


class FrameSplitter:
    """Cuts a byte stream into complete length-prefixed frames."""

    def __init__(self) -> None:
        self.buf = bytearray()

    def feed(self, data: bytes) -> list[bytes]:
        self.buf += data
        frames = []
        while True:
            header = read_varint(self.buf, 0)
            if header is None or len(self.buf) < header[1] + header[0]:
                return frames
            end = header[1] + header[0]
            frames.append(bytes(self.buf[:end]))
            del self.buf[:end]


def frame_packet(frame: bytes, compressed: bool) -> tuple[int, bytes]:
    _, pos = read_varint(frame, 0)
    body = frame[pos:]
    if compressed:
        data_len, pos = read_varint(body, 0)
        body = zlib.decompress(body[pos:]) if data_len else body[pos:]
    packet_id, pos = read_varint(body, 0)
    return packet_id, body[pos:]


class SessionLog:
    """Decodes serverbound frames into (phase, id, payload) records."""

    def __init__(self, compression: bool) -> None:
        self.compression = compression
        self.phase = "handshake"
        self.records: list[dict[str, Any]] = []

    def add(self, frame: bytes, window: int) -> None:
        compressed = self.compression and len(self.records) >= 2
        packet_id, payload = frame_packet(frame, compressed)
        phase = self.phase
        if phase == "handshake":
            # Intention: version, host, u16 port, next state. The port is the
            # run's own listener, never behaviour.
            payload = payload[:-3] + b"\0\0" + payload[-1:]
            self.phase = "login"
        elif phase == "login" and packet_id == SB_LOGIN_ACKNOWLEDGED:
            self.phase = "configuration"
        elif phase == "configuration" and packet_id == SB_FINISH_CONFIGURATION:
            self.phase = "play"
        elif phase == "play" and packet_id == SB_CONFIGURATION_ACKNOWLEDGED:
            self.phase = "configuration"
        self.records.append(
            {"window": window, "phase": phase, "id": packet_id, "payload": payload.hex()}
        )


def detect_compression(frames: list[bytes]) -> bool:
    """True when the login phase switched compression on."""
    for frame in frames[:4]:
        packet_id, _ = frame_packet(frame, False)
        if packet_id == CB_LOGIN_COMPRESSION:
            return True
    return False


# --- capture file --------------------------------------------------------------


def write_capture(path: Path, windows: dict[int, list[bytes]]) -> None:
    with path.open("wb") as out:
        for window in sorted(windows):
            for frame in windows[window]:
                out.write(struct.pack("<II", window, len(frame)))
                out.write(frame)


def read_capture(path: Path) -> dict[int, list[bytes]]:
    data = path.read_bytes()
    windows: dict[int, list[bytes]] = {}
    pos = 0
    while pos < len(data):
        window, length = struct.unpack_from("<II", data, pos)
        pos += 8
        windows.setdefault(window, []).append(data[pos : pos + length])
        pos += length
    return windows


# --- processes -----------------------------------------------------------------


@dataclass
class Tools:
    target: Path
    transport: str = "control"
    software_gpu: bool = field(default_factory=lambda: os.environ.get("BBB_SOFTWARE_GPU") == "1")
    vanilla_root: Path | None = None

    def bin(self, name: str) -> Path:
        candidates = [self.target / name]
        if os.name == "nt" and not name.endswith(".exe"):
            candidates.insert(0, self.target / f"{name}.exe")
        for cand in candidates:
            resolved = cand.resolve()
            if resolved.is_file():
                return resolved
        raise ReplayError(
            f"missing {(self.target / name).resolve()}; build bbb-client/bbb-client-probe/bbb-server/bbb-admin bins"
        )


class Weston:
    def __init__(self, out: Path) -> None:
        self._owned_runtime_dir: str | None = None
        env_runtime = os.environ.get("XDG_RUNTIME_DIR")
        if env_runtime:
            self.runtime = env_runtime
        elif hasattr(os, "getuid") and Path(f"/run/user/{os.getuid()}").is_dir():
            self.runtime = f"/run/user/{os.getuid()}"
        else:
            self._owned_runtime_dir = tempfile.mkdtemp(prefix="bbb-xdg-")
            os.chmod(self._owned_runtime_dir, 0o700)
            self.runtime = self._owned_runtime_dir
        while True:
            self.socket = f"bbb-replay-{os.getpid()}-{uuid.uuid4().hex}"
            path = Path(self.runtime, self.socket)
            if not path.exists() and not Path(f"{path}.lock").exists():
                break
        with (out / "weston.log").open("wb") as log:
            self.process = subprocess.Popen(
                [
                    "weston",
                    "--backend=headless-backend.so",
                    "--use-gl",
                    f"--socket={self.socket}",
                    f"--width={WIDTH}",
                    f"--height={HEIGHT}",
                    "--idle-time=0",
                ],
                stdout=log,
                stderr=subprocess.STDOUT,
                env={**os.environ, "XDG_RUNTIME_DIR": self.runtime},
            )
        deadline = time.monotonic() + 10
        while True:
            if self.process.poll() is not None or time.monotonic() > deadline:
                self.close()
                raise ReplayError(f"weston did not start; see {out / 'weston.log'}")
            try:
                if hasattr(socket, "AF_UNIX"):
                    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as probe:
                        probe.connect(str(path))
                    break
                time.sleep(0.05)
            except OSError:
                time.sleep(0.05)

    def close(self) -> None:
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait()
        path = Path(self.runtime, self.socket)
        path.unlink(missing_ok=True)
        Path(f"{path}.lock").unlink(missing_ok=True)
        if self._owned_runtime_dir is not None:
            shutil.rmtree(self._owned_runtime_dir, ignore_errors=True)
            self._owned_runtime_dir = None


class NativeDisplay:
    runtime: str | None = None
    socket: str | None = None

    def close(self) -> None:
        pass


def display_session(out: Path) -> Weston | NativeDisplay:
    if sys.platform.startswith("linux"):
        return Weston(out)
    return NativeDisplay()


class Client:
    def __init__(
        self,
        tools: Tools,
        out: Path,
        weston: Weston | NativeDisplay,
        server_port: int,
        scenario: Scenario,
    ):
        self.out = out
        self.transport = tools.transport
        self.port = free_port()
        self.mcp_session: str | None = None
        self.mcp_version = "2025-06-18"
        self.mcp_id = 0
        self.mcp_http = http.client.HTTPConnection("127.0.0.1", self.port, timeout=60)
        self.last_content: list[dict[str, Any]] = []
        self.game = out / "game"
        shutil.rmtree(self.game, ignore_errors=True)
        self.game.mkdir(parents=True)
        self.log = out / "client.log"
        if weston.runtime is not None and weston.socket is not None:
            env = {k: v for k, v in os.environ.items() if k != "DISPLAY"}
            env.update(
                XDG_RUNTIME_DIR=weston.runtime,
                WAYLAND_DISPLAY=weston.socket,
            )
        else:
            env = os.environ.copy()
        env.update(
            ALSOFT_DRIVERS="null",
            RUST_LOG=os.environ.get("RUST_LOG", "warn"),
        )
        if tools.software_gpu:
            env["BBB_SOFTWARE_GPU"] = "1"
            if sys.platform.startswith("linux"):
                env.setdefault("LIBGL_ALWAYS_SOFTWARE", "1")
        vanilla_dir = tools.vanilla_root or default_vanilla_root()
        with self.log.open("wb") as log_file:
            self.process = subprocess.Popen(
                [
                    str(tools.bin("bbb-client")),
                    "--game-dir",
                    str(self.game),
                    "--vanilla-root",
                    str(vanilla_dir),
                    "--server",
                    f"127.0.0.1:{server_port}",
                    "--username",
                    USERNAME,
                    "--connect-server",
                    "--mcp-bind" if self.transport == "mcp" else "--control",
                    f"127.0.0.1:{self.port}",
                    "--lockstep",
                    "--lockstep-audio-log",
                    str(out / "audio.jsonl"),
                    "--vsync",
                    "false",
                    "--pause-on-lost-focus",
                    "false",
                    "--width",
                    str(WIDTH),
                    "--height",
                    str(HEIGHT),
                    "--render-distance",
                    str(scenario.render_distance),
                    *scenario.client_args,
                ],
                stdout=log_file,
                stderr=subprocess.STDOUT,
                env=env,
            )

    def call(self, method: str, params: dict[str, Any] | None = None) -> Any:
        if self.transport == "mcp":
            if self.mcp_session is None:
                initialized, headers = self._mcp_request(
                    "initialize",
                    {
                        "protocolVersion": self.mcp_version,
                        "capabilities": {},
                        "clientInfo": {"name": "bbb-client-replay", "version": "1"},
                    },
                )
                session = headers.get("mcp-session-id")
                version = (
                    initialized.get("protocolVersion") if isinstance(initialized, dict) else None
                )
                if not session or not isinstance(version, str):
                    raise ReplayError("MCP initialize omitted session or protocol version")
                self.mcp_session, self.mcp_version = session, version
                self._mcp_request("notifications/initialized", notification=True)
            result, _ = self._mcp_request("tools/call", {"name": method, "arguments": params or {}})
            if not isinstance(result, dict) or result.get("isError"):
                raise ReplayError(f"MCP {method} {params}: tool failed: {result}")
            if "structuredContent" not in result:
                raise ReplayError(f"MCP {method}: missing structuredContent")
            self.last_content = result.get("content", [])
            return result["structuredContent"]
        with socket.create_connection(("127.0.0.1", self.port), timeout=30) as conn:
            conn.sendall(json.dumps({"method": method, "params": params}).encode() + b"\n")
            buf = b""
            while not buf.endswith(b"\n"):
                chunk = conn.recv(65536)
                if not chunk:
                    break
                buf += chunk
        answer = json.loads(buf)
        if not answer.get("ok"):
            raise ReplayError(f"{method} {params}: {answer.get('error')}")
        return answer.get("result")

    def _mcp_request(
        self, method: str, params: dict[str, Any] | None = None, *, notification: bool = False
    ) -> tuple[Any, dict[str, str]]:
        message: dict[str, Any] = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            message["params"] = params
        if not notification:
            self.mcp_id += 1
            message["id"] = self.mcp_id
        headers = {"Content-Type": "application/json", "Accept": "application/json"}
        if self.mcp_session is not None:
            headers["Mcp-Session-Id"] = self.mcp_session
            headers["Mcp-Protocol-Version"] = self.mcp_version
        body_json = json.dumps(message)
        reused_keepalive = self.mcp_http.sock is not None
        try:
            self.mcp_http.request("POST", "/mcp", body_json, headers)
            response = self.mcp_http.getresponse()
            body = response.read()
        except (OSError, http.client.HTTPException) as error:
            self.mcp_http.close()
            if not reused_keepalive or not isinstance(
                error, (ConnectionError, http.client.BadStatusLine)
            ):
                raise ReplayError(f"MCP {method}: HTTP transport failed: {error}") from error
            try:
                self.mcp_http.request("POST", "/mcp", body_json, headers)
                response = self.mcp_http.getresponse()
                body = response.read()
            except (OSError, http.client.HTTPException) as retry_error:
                self.mcp_http.close()
                raise ReplayError(
                    f"MCP {method}: HTTP transport failed: {retry_error}"
                ) from retry_error
        if not 200 <= response.status < 300:
            raise ReplayError(f"MCP {method}: HTTP {response.status}: {body[:1000]!r}")
        if notification:
            return None, dict(response.getheaders())
        try:
            answer = json.loads(body)
        except (ValueError, UnicodeDecodeError) as error:
            raise ReplayError(f"MCP {method}: invalid JSON response") from error
        if (
            not isinstance(answer, dict)
            or answer.get("jsonrpc") != "2.0"
            or answer.get("id") != self.mcp_id
        ):
            raise ReplayError(f"MCP {method}: invalid JSON-RPC response: {answer}")
        if "error" in answer or "result" not in answer:
            raise ReplayError(f"MCP {method}: JSON-RPC error: {answer.get('error')}")
        return answer["result"], {name.lower(): value for name, value in response.getheaders()}

    def wait_control(self) -> None:
        deadline = time.monotonic() + 120
        while True:
            self.check_alive()
            try:
                self.call("sim.lockstep_status")
                return
            except (OSError, ReplayError) as error:
                if isinstance(error, ReplayError) and not isinstance(error.__cause__, OSError):
                    raise
                if time.monotonic() > deadline:
                    raise ReplayError(f"client {self.transport} never came up; see {self.log}")
                time.sleep(0.1)

    def check_alive(self) -> None:
        if self.process.poll() is not None:
            tail = self.log.read_text(errors="replace")[-3000:]
            raise ReplayError(f"client exited {self.process.returncode}:\n{tail}")

    def quit(self) -> None:
        try:
            if self.process.poll() is None:
                try:
                    self.call("app.quit")
                except (OSError, ReplayError):
                    pass
                try:
                    self.process.wait(timeout=60)
                except subprocess.TimeoutExpired as error:
                    self.process.kill()
                    self.process.wait()
                    raise ReplayError(f"client did not quit; see {self.log}") from error
            if self.process.returncode != 0:
                raise ReplayError(f"client exited {self.process.returncode}; see {self.log}")
        finally:
            self.mcp_http.close()


class Link:
    """The client's one game connection, seen from our listener."""

    def __init__(self, upstream_port: int | None) -> None:
        self.listener = socket.create_server(("127.0.0.1", 0))
        self.port = self.listener.getsockname()[1]
        self.upstream_port = upstream_port
        self.client: socket.socket | None = None
        self.server: socket.socket | None = None
        self.lock = threading.Lock()
        self.serverbound: list[bytes] = []
        self.held: list[bytes] = []
        self.closed = False

    def accept(self, timeout: float = 120) -> None:
        self.listener.settimeout(timeout)
        self.client, _ = self.listener.accept()
        self.client.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        if self.upstream_port is not None:
            self.server = socket.create_connection(("127.0.0.1", self.upstream_port))
            self.server.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            threading.Thread(target=self._pump_server, daemon=True).start()
        threading.Thread(target=self._pump_client, daemon=True).start()

    def _pump_client(self) -> None:
        splitter = FrameSplitter()
        while data := self._recv(self.client):
            if self.server is not None:
                self.server.sendall(data)
            frames = splitter.feed(data)
            with self.lock:
                self.serverbound += frames
        self.closed = True

    def _pump_server(self) -> None:
        splitter = FrameSplitter()
        while data := self._recv(self.server):
            frames = splitter.feed(data)
            with self.lock:
                self.held += frames

    @staticmethod
    def _recv(conn: socket.socket | None) -> bytes:
        try:
            return conn.recv(1 << 20) if conn else b""
        except OSError:
            return b""

    def take_held(self) -> list[bytes]:
        with self.lock:
            frames, self.held = self.held, []
        return frames

    def deliver(self, frames: list[bytes]) -> None:
        if frames:
            self.client.sendall(b"".join(frames))

    def serverbound_count(self) -> int:
        with self.lock:
            return len(self.serverbound)

    def close(self) -> None:
        for conn in (self.client, self.server, self.listener):
            if conn is not None:
                try:
                    conn.close()
                except OSError:
                    pass


# --- scenarios -----------------------------------------------------------------


@dataclass
class Scenario:
    name: str
    steps: int
    world: Path | None = None
    render_distance: int = 6
    client_args: list[str] = field(default_factory=list)
    #: The window a first click grabs the cursor, as a player clicking into
    #: the game window would; world keys and look input need the grab.
    grab_at: int = 100
    actions: dict[int, list[tuple[str, Any]]] = field(default_factory=dict)
    #: Screenshot regions left out of the comparison, per label: pixels that
    #: show wall-clock state no replay can pin (the F3 frame rate).
    masks: dict[str, list[tuple[int, int, int, int]]] = field(default_factory=dict)

    def __post_init__(self) -> None:
        self.control(self.grab_at, "input.mouse_tap", button="left")

    def at(self, step: int, kind: str, value: Any = None) -> Scenario:
        self.actions.setdefault(step, []).append((kind, value))
        return self

    def server(self, step: int, *commands: str) -> Scenario:
        for command in commands:
            self.at(step, "server", command)
        return self

    def shot(
        self, step: int, label: str, masks: list[tuple[int, int, int, int]] | None = None
    ) -> Scenario:
        if masks:
            self.masks[label] = masks
        return self.at(step, "shot", label)

    def control(self, step: int, method: str, **params: Any) -> Scenario:
        return self.at(step, "control", [method, params])

    def key(self, step: int, key: str, ticks: int = 1) -> Scenario:
        self.control(step, "input.key", key=key, pressed=True)
        return self.control(step + ticks, "input.key", key=key, pressed=False)

    def look(self, step: int, dx: float, dy: float) -> Scenario:
        return self.control(step, "input.look_delta", dx=dx, dy=dy)


def scenario_plan(scenario: Scenario) -> dict[str, Any]:
    plan = {
        "version": 1,
        "name": scenario.name,
        "steps": scenario.steps,
        "render_distance": scenario.render_distance,
        "client_args": scenario.client_args,
        "actions": [
            {"window": window, "kind": kind, "value": value}
            for window in sorted(scenario.actions)
            for kind, value in scenario.actions[window]
        ],
        "masks": scenario.masks,
    }
    return json.loads(json.dumps(plan, allow_nan=False))


def write_plan(directory: Path, scenario: Scenario) -> None:
    (directory / "scenario.json").write_text(json.dumps(scenario_plan(scenario), indent=2) + "\n")


def read_plan(directory: Path, name: str) -> Scenario | None:
    path = directory / "scenario.json"
    if not path.exists():
        return None
    try:
        plan = json.loads(path.read_text())
        json.dumps(plan, allow_nan=False)
        fields = {"version", "name", "steps", "render_distance", "client_args", "actions", "masks"}
        if not isinstance(plan, dict) or set(plan) != fields:
            raise ValueError(
                "expected version/name/steps/render_distance/client_args/actions/masks"
            )
        if type(plan["version"]) is not int or plan["version"] != 1:
            raise ValueError(f"unsupported version {plan['version']!r}")
        if plan["name"] != name:
            raise ValueError(f"expected scenario {name!r}, found {plan['name']!r}")
        for field_ in ("steps", "render_distance"):
            if type(plan[field_]) is not int or plan[field_] <= 0:
                raise ValueError(f"{field_} must be a positive integer")
        if not isinstance(plan["client_args"], list) or not all(
            isinstance(arg, str) for arg in plan["client_args"]
        ):
            raise ValueError("client_args must be a list of strings")
        if not isinstance(plan["actions"], list):
            raise ValueError("actions must be an ordered list")
        actions: dict[int, list[tuple[str, Any]]] = {}
        previous = -1
        for action in plan["actions"]:
            if not isinstance(action, dict) or set(action) != {"window", "kind", "value"}:
                raise ValueError("each action needs window/kind/value")
            window, kind, value = action["window"], action["kind"], action["value"]
            if type(window) is not int or window < 0 or window < previous:
                raise ValueError("action windows must be nonnegative and ordered")
            previous = window
            if kind in ("server", "shot"):
                if not isinstance(value, str) or not value:
                    raise ValueError(f"{kind} value must be a nonempty string")
                if kind == "shot" and (Path(value).name != value or value in (".", "..")):
                    raise ValueError("shot label must be a filename")
            elif kind == "control":
                if (
                    not isinstance(value, list)
                    or len(value) != 2
                    or not isinstance(value[0], str)
                    or not value[0]
                    or not isinstance(value[1], dict)
                ):
                    raise ValueError("control value must be [method, params]")
            else:
                raise ValueError(f"unknown action kind {kind!r}")
            actions.setdefault(window, []).append((kind, value))
        masks = plan["masks"]
        if not isinstance(masks, dict):
            raise ValueError("masks must be an object")
        for rectangles in masks.values():
            if not isinstance(rectangles, list) or any(
                not isinstance(rect, list)
                or len(rect) != 4
                or any(type(v) is not int for v in rect)
                for rect in rectangles
            ):
                raise ValueError("masks must contain lists of four-integer rectangles")
        scenario = Scenario(
            name, plan["steps"],
            render_distance=plan["render_distance"], client_args=plan["client_args"],
        )
        scenario.actions, scenario.masks = actions, masks
        return scenario
    except (OSError, ValueError, TypeError) as error:
        raise ReplayError(f"invalid scenario plan {path}: {error}") from error


JOIN = 120  # windows until a flat join has settled
FLOOR = 64  # first air block above the flat fixture's floor
F3_FPS_LINE = (0, 18, 200, 38)  # the F3 "<n> fps" line at the default GUI scale


def flat_idle() -> Scenario:
    s = Scenario("flat_idle", JOIN + 80)
    return s.shot(JOIN, "settled").shot(JOIN + 79, "later")


def flat_move() -> Scenario:
    s = Scenario("flat_move", JOIN + 160)
    t = JOIN
    s.key(t, "forward", 30).key(t + 10, "sprint", 15).key(t + 20, "jump", 2)
    s.look(t + 35, 300, 0).key(t + 40, "backward", 20).key(t + 45, "sneak", 15)
    s.key(t + 65, "left", 10).key(t + 75, "right", 10).look(t + 85, -150, 80)
    s.key(t + 90, "hotbar3").key(t + 95, "hotbar7")
    s.shot(t + 30, "sprint").shot(t + 60, "sneak").shot(t + 100, "look_down")
    s.key(t + 110, "jump", 2).key(t + 111, "jump", 2)  # double tap: creative flight
    s.key(t + 115, "jump", 20).shot(t + 140, "flying")
    return s


def flat_ui() -> Scenario:
    s = Scenario("flat_ui", JOIN + 200)
    t = JOIN
    s.server(t - 20, "give @a minecraft:diamond_sword", "give @a minecraft:oak_log 32")
    s.key(t, "inventory").shot(t + 5, "inventory").key(t + 8, "escape")
    s.key(t + 12, "chat").control(t + 14, "input.text", text="hello replay")
    s.shot(t + 16, "chat_typing").key(t + 18, "enter").shot(t + 22, "chat_sent")
    s.key(t + 30, "escape").shot(t + 34, "pause_menu").key(t + 38, "escape")
    s.key(t + 45, "f3").shot(t + 48, "debug", masks=[F3_FPS_LINE]).key(t + 52, "f3")
    s.key(t + 58, "f5").shot(t + 62, "third_person").key(t + 66, "f5").shot(t + 70, "front_view")
    s.key(t + 74, "f5").key(t + 80, "f1").shot(t + 83, "hidden_hud").key(t + 86, "f1")
    s.control(t + 90, "input.key", key="tab", pressed=True).shot(t + 93, "player_list")
    s.control(t + 95, "input.key", key="tab", pressed=False)
    s.key(t + 100, "command").control(t + 102, "input.text", text="time set ")
    s.shot(t + 105, "command_suggestions").key(t + 108, "escape")
    s.key(t + 115, "advancements").shot(t + 120, "advancements").key(t + 124, "escape")
    s.key(t + 130, "drop").key(t + 135, "swap_offhand").shot(t + 145, "dropped_item")
    return s


def flat_entities() -> Scenario:
    s = Scenario("flat_entities", JOIN + 120)
    mobs = [
        "pig",
        "cow",
        "sheep",
        "chicken",
        "wolf",
        "cat",
        "fox",
        "horse",
        "villager",
        "zombie",
        "skeleton",
        "creeper",
        "spider",
        "enderman",
        "iron_golem",
        "slime",
        "armor_stand",
        "allay",
        "axolotl",
        "frog",
        "camel",
        "sniffer",
        "panda",
        "llama",
    ]
    # Hostile mobs need a difficulty above the fixture's peaceful.
    commands = ["difficulty easy", f"tp @a 0 {FLOOR} 0 0 0"]
    for index, mob in enumerate(mobs):
        x = (index % 6) * 3 - 7.5
        z = 6 + (index // 6) * 3
        commands.append(f"summon minecraft:{mob} {x} {FLOOR} {z} {{NoAI:1b,Silent:1b}}")
    s.server(JOIN - 40, *commands)
    s.shot(JOIN + 10, "grid").look(JOIN + 20, 0, 25).shot(JOIN + 25, "grid_low")
    s.look(JOIN + 40, 400, 0).shot(JOIN + 45, "grid_side")
    s.server(
        JOIN + 60,
        f'summon minecraft:item 1 {FLOOR + 1} 3 {{Item:{{id:"minecraft:diamond",count:1}}}}',
    )
    s.server(JOIN + 60, f"summon minecraft:experience_orb 2 {FLOOR + 1} 3 {{Value:7}}")
    s.look(JOIN + 62, -400, 0).shot(JOIN + 80, "items").shot(JOIN + 119, "items_later")
    return s


def flat_blocks() -> Scenario:
    s = Scenario("flat_blocks", JOIN + 80)
    blocks = [
        "chest[facing=north]",
        "oak_sign[rotation=8]",
        "red_bed[facing=east,part=foot]",
        "skeleton_skull",
        "white_banner",
        "lantern",
        "torch",
        "oak_fence",
        "glass",
        "water",
        "lava",
        "grass_block",
        "oak_leaves",
        "redstone_wire",
        "rail",
        "anvil",
        "brewing_stand",
        "enchanting_table",
        "end_portal_frame",
        "bell",
        "campfire",
        "decorated_pot",
        "beacon",
        "flower_pot",
    ]
    commands = [f"tp @a 0 {FLOOR} -2 0 20"]
    for index, block in enumerate(blocks):
        x = (index % 8) * 2 - 7
        z = 3 + (index // 8) * 2
        commands.append(f"setblock {x} {FLOOR} {z} minecraft:{block}")
    s.server(JOIN - 40, *commands)
    s.shot(JOIN + 10, "showcase").look(JOIN + 20, 200, 0).shot(JOIN + 25, "showcase_right")
    s.look(JOIN + 40, -400, 0).shot(JOIN + 45, "showcase_left").shot(JOIN + 79, "later")
    return s


def flat_environment() -> Scenario:
    s = Scenario("flat_environment", JOIN + 160)
    t = JOIN
    s.server(t, "time set 13000").shot(t + 5, "dusk")
    s.server(t + 10, "time set 18000").shot(t + 15, "night")
    s.server(t + 20, "time set 1000", "weather rain").shot(t + 60, "rain")
    s.server(t + 70, "weather thunder").shot(t + 100, "thunder")
    s.server(t + 110, "weather clear", f"particle minecraft:flame 0 {FLOOR + 1} 4 1 1 1 0.02 60")
    s.server(t + 111, f"particle minecraft:heart 2 {FLOOR + 1} 4 1 1 1 0 20")
    s.shot(t + 115, "particles").shot(t + 130, "particles_later")
    return s


def flat_hud() -> Scenario:
    s = Scenario("flat_hud", JOIN + 160)
    t = JOIN
    s.server(
        t,
        'bossbar add replay:bar "Replay Bar"',
        "bossbar set replay:bar players @a",
        "bossbar set replay:bar value 40",
        'scoreboard objectives add replay dummy "Sidebar"',
        "scoreboard objectives setdisplay sidebar replay",
        f"scoreboard players set {USERNAME} replay 7",
        'title @a title "Big Title"',
        'title @a subtitle "small subtitle"',
    )
    s.shot(t + 10, "title_bossbar")
    s.server(t + 20, 'title @a actionbar "action bar"').shot(t + 25, "actionbar")
    s.server(t + 40, "gamemode survival @a", "xp add @a 30 levels", "difficulty easy")
    s.shot(t + 50, "survival_hud")
    s.server(t + 60, "effect give @a minecraft:poison 10 1", "effect give @a minecraft:speed 30 1")
    s.shot(t + 90, "effects")
    s.server(t + 100, "damage @a 4 minecraft:generic").shot(t + 103, "hurt")
    s.server(t + 120, "kill @a").shot(t + 150, "death_screen")
    return s


def flat_containers() -> Scenario:
    s = Scenario("flat_containers", JOIN + 160)
    items = (
        '[{Slot:0b,id:"minecraft:diamond",count:5},{Slot:4b,id:"minecraft:oak_log",count:32},'
        '{Slot:13b,id:"minecraft:iron_sword",count:1}]'
    )
    s.server(
        JOIN - 40,
        f"setblock 0 {FLOOR} 2 minecraft:chest[facing=north]{{Items:{items}}}",
        f"setblock 2 {FLOOR} 2 minecraft:crafting_table",
        f"setblock -2 {FLOOR} 2 minecraft:furnace[facing=north]",
        f"setblock 2 {FLOOR} -2 minecraft:anvil",
        f"setblock -2 {FLOOR} -2 minecraft:enchanting_table",
        "give @a minecraft:apple 3",
        f"tp @a 0.5 {FLOOR} 0.5 0 30",
    )
    # Each empty-handed right click opens the block in front and escape closes
    # it. Closing leaves the cursor released, so a click at the sky (nothing in
    # reach) grabs it again before the next block.
    t = JOIN
    s.key(t - 10, "hotbar2")
    for step, label, yaw, pitch in (
        (t, "chest", 0, 30),
        (t + 15, "crafting", -37, 24),
        (t + 30, "furnace", 37, 24),
        (t + 45, "anvil", -135, 22),
    ):
        s.server(step - 3, f"tp @a 0.5 {FLOOR} 0.5 {yaw} {pitch}")
        s.control(step, "input.mouse_tap", button="right").shot(step + 5, label)
        s.key(step + 8, "escape").server(step + 9, f"tp @a 0.5 {FLOOR} 0.5 {yaw} -80")
        s.control(step + 11, "input.mouse_tap", button="right")
    s.server(t + 57, f"tp @a 0.5 {FLOOR} 0.5 135 22").shot(t + 62, "enchanting_table")
    s.server(t + 104, f"tp @a 0.5 {FLOOR} 0.5 90 10")
    s.key(t + 106, "hotbar1").control(t + 108, "input.mouse_button", button="right", pressed=True)
    s.shot(t + 118, "eating").control(t + 150, "input.mouse_button", button="right", pressed=False)
    s.shot(t + 155, "after_eating")
    return s


def flat_bestiary() -> Scenario:
    """Every summonable entity kind in six posed groups, each shot twice."""
    s = Scenario("flat_bestiary", JOIN + 200)
    mob = "{NoAI:1b,Silent:1b,PersistenceRequired:1b}"
    baby = "{NoAI:1b,Silent:1b,PersistenceRequired:1b,Age:-24000}"
    baby_zombie = "{NoAI:1b,Silent:1b,PersistenceRequired:1b,IsBaby:1b}"
    still = "{NoGravity:1b}"
    armor = (
        '{ShowArms:1b,equipment:{head:{id:"diamond_helmet"},chest:{id:"iron_chestplate"},'
        'legs:{id:"golden_leggings"},feet:{id:"leather_boots"},mainhand:{id:"iron_sword"}}}'
    )
    groups: list[tuple[int, int, int, list[tuple[str, str]]]] = [
        (
            -36,
            10,
            3,
            [
                (name, mob)
                for name in (
                    "pig",
                    "cow",
                    "sheep",
                    "chicken",
                    "mooshroom",
                    "rabbit",
                    "goat",
                    "llama",
                    "trader_llama",
                    "horse",
                    "donkey",
                    "mule",
                    "skeleton_horse",
                    "zombie_horse",
                    "camel",
                    "camel_husk",
                    "panda",
                    "polar_bear",
                    "fox",
                    "wolf",
                )
            ],
        ),
        (
            0,
            10,
            3,
            [
                (name, mob)
                for name in (
                    "cat",
                    "ocelot",
                    "parrot",
                    "bee",
                    "allay",
                    "bat",
                    "frog",
                    "tadpole",
                    "axolotl",
                    "turtle",
                    "cod",
                    "salmon",
                    "pufferfish",
                    "tropical_fish",
                    "squid",
                    "glow_squid",
                    "dolphin",
                    "armadillo",
                    "strider",
                    "snow_golem",
                )
            ],
        ),
        (
            36,
            10,
            3,
            [
                (name, mob)
                for name in (
                    "zombie",
                    "husk",
                    "drowned",
                    "zombie_villager",
                    "skeleton",
                    "stray",
                    "bogged",
                    "parched",
                    "wither_skeleton",
                    "piglin",
                    "piglin_brute",
                    "zombified_piglin",
                    "pillager",
                    "vindicator",
                    "evoker",
                    "illusioner",
                    "witch",
                    "vex",
                    "creeper",
                    "villager",
                )
            ],
        ),
        (
            -36,
            40,
            3,
            [
                (name, mob)
                for name in (
                    "spider",
                    "cave_spider",
                    "enderman",
                    "endermite",
                    "silverfish",
                    "slime",
                    "magma_cube",
                    "blaze",
                    "breeze",
                    "creaking",
                    "guardian",
                    "phantom",
                    "shulker",
                    "hoglin",
                    "zoglin",
                    "wandering_trader",
                    "copper_golem",
                    "nautilus",
                    "zombie_nautilus",
                    "mannequin",
                )
            ],
        ),
        (
            36,
            40,
            3,
            [
                ("armor_stand", armor),
                ("oak_boat", still),
                ("oak_chest_boat", still),
                ("bamboo_raft", still),
                ("minecart", still),
                ("chest_minecart", still),
                ("furnace_minecart", still),
                ("hopper_minecart", still),
                ("tnt_minecart", still),
                ("spawner_minecart", still),
                ("command_block_minecart", still),
                ("end_crystal", "{ShowBottom:1b}"),
                ("falling_block", '{NoGravity:1b,Time:1,BlockState:{Name:"minecraft:sand"}}'),
                ("tnt", "{fuse:32767s}"),
                ("block_display", '{block_state:{Name:"minecraft:oak_log"}}'),
                ("item_display", '{item:{id:"minecraft:diamond_sword",count:1}}'),
                ("text_display", '{text:"bestiary",billboard:"center"}'),
                ("arrow", still),
                ("trident", still),
                ("snowball", still),
            ],
        ),
        (
            0,
            40,
            10,
            [
                (name, mob)
                for name in (
                    "iron_golem",
                    "ravager",
                    "sniffer",
                    "warden",
                    "elder_guardian",
                    "ghast",
                    "happy_ghast",
                    "wither",
                    "giant",
                    "ender_dragon",
                )
            ],
        ),
    ]
    babies = [
        ("pig", baby),
        ("cow", baby),
        ("chicken", baby),
        ("wolf", baby),
        ("villager", baby),
        ("armadillo", baby),
        ("axolotl", baby),
        ("camel", baby),
        ("zombie", baby_zombie),
        ("piglin", baby_zombie),
    ]
    commands = ["difficulty easy", "gamerule do_daylight_cycle false", "time set 6000"]
    for ox, oz, spacing, entries in groups:
        for index, (name, nbt) in enumerate(entries):
            x = ox + (index % 5) * spacing - 2 * spacing
            z = oz + (index // 5) * (spacing + 1)
            commands.append(f"summon minecraft:{name} {x}.5 {FLOOR} {z}.5 {nbt}")
    # Babies and wall-mounted entities stand south of spawn, facing a camera
    # that looks north; the frames and painting hang on a stone wall at z=-14.
    for index, (name, nbt) in enumerate(babies):
        commands.append(f"summon minecraft:{name} {index * 2 - 9}.5 {FLOOR} -9.5 {nbt}")
    commands += [
        f"fill -10 {FLOOR} -14 10 {FLOOR + 2} -14 minecraft:stone",
        f"summon minecraft:item_frame -6.5 {FLOOR + 1} -12.5 "
        '{Facing:3b,Item:{id:"minecraft:compass",count:1}}',
        f"summon minecraft:glow_item_frame -4.5 {FLOOR + 1} -12.5 "
        '{Facing:3b,Item:{id:"minecraft:clock",count:1}}',
        f'summon minecraft:painting 0 {FLOOR + 1} -13 {{Facing:3b,variant:"minecraft:kebab"}}',
        f"setblock 6 {FLOOR} -12 minecraft:oak_fence",
        f"summon minecraft:leash_knot 6 {FLOOR} -12",
    ]
    s.server(JOIN - 50, *commands)
    views = [
        (-36, 5, 3, 0),
        (0, 5, 3, 0),
        (36, 5, 3, 0),
        (-36, 5, 33, 0),
        (36, 5, 33, 0),
        (0, 4, -2, 180),
        (0, 16, 22, 0),
    ]
    for index, (x, height, z, yaw) in enumerate(views):
        step = JOIN + index * 28
        s.server(step, f"tp @a {x}.5 {FLOOR + height} {z}.5 {yaw} 25")
        s.shot(step + 6, f"group{index}").shot(step + 26, f"group{index}_later")
    return s


def gui_px(x: int, y: int) -> dict[str, float]:
    """Window pixel centre of a GUI-scale-2 point (854x480 window, 427x240 GUI)."""
    return {"x": x * 2 + 1.0, "y": y * 2 + 1.0}


def flat_menus() -> Scenario:
    """Block-opened menus and editors, each set at one spot the camera aims at."""
    blocks = [
        ('command_block{Command:"say replay"}', " again"),
        ('structure_block{name:"replay:house",mode:"SAVE"}', "_v2"),
        ('jigsaw{name:"replay:start",pool:"replay:pool"}', "_v2"),
        ('test_block{mode:"log",message:"replay"}', " message"),
        ("test_instance_block", None),
        ("loom", None),
        ("stonecutter", None),
        ("smithing_table", None),
        ("cartography_table", None),
        ("grindstone", None),
        ("beacon", None),
        ("brewing_stand", None),
        ("blast_furnace", None),
        ("smoker", None),
        ("barrel", None),
        ("hopper", None),
        ("dispenser", None),
        ("dropper", None),
        ("shulker_box", None),
        ("crafter", None),
        (
            'lectern[has_book=true]{Book:{id:"minecraft:written_book",count:1,components:'
            '{"minecraft:written_book_content":{title:"Replay",author:"bbb",'
            'pages:["First page","Second page"]}}}}',
            None,
        ),
        ("anvil", "Renamed"),
    ]
    spacing = 18
    s = Scenario("flat_menus", JOIN + 20 + spacing * len(blocks))
    s.server(
        JOIN - 50,
        "op bbb-replay",
        "gamerule advance_time false",
        "time set 6000",
        f"tp @a 0.5 {FLOOR} 0.5 0 30",
    )
    # Each right click opens the block in front and escape closes it. Closing
    # leaves the cursor released (the headless window is not focused), so a
    # click at the sky grabs it again before the next block.
    for index, (block, text) in enumerate(blocks):
        step = JOIN + 10 + index * spacing
        name = block.split("[")[0].split("{")[0]
        if name == "anvil":
            # A sword in the anvil's input slot enables the rename field.
            s.server(step - 8, "give bbb-replay minecraft:iron_sword")
        s.server(step - 6, f"setblock 0 {FLOOR} 2 minecraft:{block}")
        s.server(step - 3, f"tp @a 0.5 {FLOOR} 0.5 0 30")
        s.control(step, "input.mouse_tap", button="right")
        if name == "command_block":
            # bbb opens the editor only once the server has sent the block
            # entity data, which the first click asks for; Vanilla opens at once.
            s.control(step + 2, "input.mouse_tap", button="right")
        if name == "anvil":
            s.control(step + 3, "input.click", button="left", **gui_px(141, 187))
            s.control(step + 4, "input.click", button="left", **gui_px(160, 92))
        if text:
            s.control(step + 5, "input.text", text=text)
        s.shot(step + 8, name)
        s.key(step + 10, "escape").server(step + 12, f"tp @a 0.5 {FLOOR} 0.5 0 -80")
        s.control(step + 14, "input.mouse_tap", button="right")
    return s


def flat_screens() -> Scenario:
    """Key-, click- and server-opened screens: social, debug options, game mode
    switcher, stats, creative search, recipe book, books, sign, dialog,
    spectator menu and an advancement toast."""
    s = Scenario("flat_screens", JOIN + 210)
    book = (
        'minecraft:written_book[written_book_content={title:"Replay",author:"bbb",'
        'pages:["First page","Second page"]}]'
    )
    sky = f"tp @a 0.5 {FLOOR} 0.5 0 -80"
    s.server(
        JOIN - 50,
        "op bbb-replay",
        "gamerule advance_time false",
        "time set 6000",
        "give bbb-replay minecraft:writable_book",
        f"give bbb-replay {book}",
        "give bbb-replay minecraft:oak_sign",
        sky,
    )
    t = JOIN
    s.key(t - 5, "hotbar9")

    def close(step: int) -> None:
        # Escape closes the screen; the released cursor is grabbed again by a
        # click at the sky (the headless window is not focused).
        s.key(step, "escape").control(step + 2, "input.mouse_tap", button="left")

    # The social screen title shows the proxy's ephemeral port.
    s.key(t, "social").shot(t + 4, "social", masks=[(190, 64, 400, 90)])
    close(t + 6)
    s.control(t + 10, "input.key", key="f3", pressed=True).key(t + 11, "f6")
    s.control(t + 12, "input.key", key="f3", pressed=False)
    s.control(t + 13, "input.text", text="fps").shot(t + 16, "debug_options")
    close(t + 18)
    s.control(t + 24, "input.key", key="f3", pressed=True).key(t + 25, "f4")
    s.shot(t + 28, "gamemode_switcher").control(t + 30, "input.key", key="f3", pressed=False)
    s.server(t + 32, "gamemode creative bbb-replay")
    s.key(t + 36, "escape").control(t + 39, "input.click", button="left", x=530, y=203)
    # Vanilla's StatsScreen.onClose returns to the pause screen; bbb returns to
    # the game. Two clicks at "Return to Game" end in game either way.
    s.shot(t + 46, "stats").key(t + 50, "escape")
    s.control(t + 52, "input.click", button="left", x=427, y=155)
    s.control(t + 54, "input.click", button="left", x=427, y=155)
    s.key(t + 58, "inventory").key(t + 61, "chat").control(t + 63, "input.text", text="oak")
    s.shot(t + 67, "creative_search")
    close(t + 70)
    s.server(t + 76, "gamemode survival bbb-replay").key(t + 80, "inventory")
    s.control(t + 83, "input.click", button="left", **gui_px(239, 107))
    s.shot(t + 88, "recipe_book")
    close(t + 91)
    s.server(t + 93, "gamemode creative bbb-replay")
    s.key(t + 98, "hotbar1").control(t + 100, "input.mouse_tap", button="right")
    s.control(t + 103, "input.text", text="Hello replay book").shot(t + 107, "book_edit")
    close(t + 110)
    s.key(t + 115, "hotbar2").control(t + 117, "input.mouse_tap", button="right")
    s.shot(t + 121, "book_view").control(t + 123, "input.click", button="left", **gui_px(244, 165))
    s.shot(t + 127, "book_page2")
    close(t + 130)
    s.key(t + 135, "hotbar3").server(t + 136, f"tp @a 0.5 {FLOOR} 0.5 0 60")
    s.control(t + 139, "input.mouse_tap", button="right")
    s.control(t + 143, "input.text", text="Replay sign").shot(t + 147, "sign_edit")
    s.key(t + 150, "escape").server(t + 152, sky).control(t + 154, "input.mouse_tap", button="left")
    s.server(
        t + 158,
        'dialog show bbb-replay {type:"minecraft:notice",title:"Replay dialog",'
        'body:{type:"minecraft:plain_message",contents:"Hello from the replay"}}',
    )
    s.shot(t + 163, "dialog")
    close(t + 166)
    s.server(t + 172, "gamemode spectator bbb-replay").key(t + 176, "hotbar1")
    s.shot(t + 180, "spectator_menu").server(t + 184, "gamemode creative bbb-replay")
    s.server(t + 190, "advancement grant bbb-replay only minecraft:story/mine_stone")
    s.shot(t + 204, "advancement_toast")
    return s


def particle_options(x: float, y: float, z: float) -> dict[str, str]:
    """Every 26.1 particle type (ParticleTypes.java) with the options its codec requires."""
    stone = '{block_state:{Name:"minecraft:stone"}}'
    plain = (
        "angry_villager bubble cloud copper_fire_flame crit damage_indicator dripping_lava "
        "falling_lava landing_lava dripping_water falling_water enchanted_hit enchant end_rod "
        "explosion_emitter explosion gust small_gust gust_emitter_large gust_emitter_small "
        "sonic_boom elder_guardian firework fishing flame infested cherry_leaves pale_oak_leaves "
        "sculk_soul sculk_charge_pop soul_fire_flame soul happy_villager composter heart "
        "pause_mob_growth "
        "reset_mob_growth item_slime item_cobweb item_snowball large_smoke lava mycelium note "
        "poof portal rain smoke white_smoke sneeze spit squid_ink sweep_attack totem_of_undying "
        "underwater splash witch bubble_pop current_down bubble_column_up nautilus dolphin "
        "campfire_cosy_smoke campfire_signal_smoke dripping_honey falling_honey landing_honey "
        "falling_nectar falling_spore_blossom ash crimson_spore warped_spore spore_blossom_air "
        "dripping_obsidian_tear falling_obsidian_tear landing_obsidian_tear reverse_portal "
        "white_ash small_flame snowflake dripping_dripstone_lava falling_dripstone_lava "
        "dripping_dripstone_water falling_dripstone_water glow_squid_ink glow wax_on wax_off "
        "electric_spark scrape egg_crack dust_plume trial_spawner_detection "
        "trial_spawner_detection_ominous vault_connection ominous_spawning raid_omen trial_omen "
        "firefly"
    ).split()
    options = {name: "" for name in plain}
    options |= {
        "block": stone,
        "block_marker": '{block_state:{Name:"minecraft:barrier"}}',
        "falling_dust": '{block_state:{Name:"minecraft:red_sand"}}',
        "dust_pillar": stone,
        "block_crumble": '{block_state:{Name:"minecraft:oak_planks"}}',
        "dragon_breath": "{power:1.0}",
        "dust": "{color:[1.0,0.0,0.0],scale:1.0}",
        "dust_color_transition": "{from_color:[1.0,0.0,0.0],to_color:[0.0,0.0,1.0],scale:1.5}",
        "effect": "{color:[1.0,0.5,0.0],power:1.0}",
        "instant_effect": "{color:[0.2,0.4,1.0],power:1.0}",
        "entity_effect": "{color:[1.0,0.0,1.0,0.0]}",
        "tinted_leaves": "{color:[1.0,0.3,0.8,0.2]}",
        "flash": "{color:[1.0,1.0,1.0,1.0]}",
        "sculk_charge": "{roll:0.5}",
        "shriek": "{delay:0}",
        "item": '{item:{id:"minecraft:apple"}}',
        "vibration": f'{{destination:{{type:"minecraft:block",pos:[0,{FLOOR + 1},0]}},'
        "arrival_in_ticks:20}",
        "trail": f"{{target:[{x + 2},{y},{z}],color:16711680,duration:20}}",
    }
    return options


def flat_particles() -> Scenario:
    """Every particle type via /particle in front of a gray wall, twelve per batch."""
    # The elder guardian curse covers the screen; it gets the last batch alone.
    names = sorted(set(particle_options(0, 0, 0)) - {"elder_guardian"})
    batches = [names[i : i + 12] for i in range(0, len(names), 12)] + [["elder_guardian"]]
    spacing = 14
    s = Scenario("flat_particles", JOIN + 10 + spacing * len(batches))
    s.server(
        JOIN - 50,
        "gamerule advance_time false",
        "time set 6000",
        "weather clear",
        f"fill -12 {FLOOR} 12 12 {FLOOR + 9} 12 minecraft:light_gray_concrete",
        f"tp @a 0.5 {FLOOR + 1} -3.5 0 0",
    )
    for index, batch in enumerate(batches):
        step = JOIN + 10 + index * spacing
        commands = []
        for slot, name in enumerate(batch):
            # Facing south +x is screen left, clear of the join toasts on the right.
            x = 7.0 - (slot % 4) * 2.5
            y = FLOOR + 0.5 + (slot // 4) * 2
            z = 6.5
            opts = particle_options(x, y, z).get(name, "")
            commands.append(f"particle minecraft:{name}{opts} {x} {y} {z} 0.3 0.3 0.3 0.02 6")
        s.server(step, *commands)
        s.shot(step + 2, f"batch{index}").shot(step + 8, f"batch{index}_later")
    return s


def flat_items_text() -> Scenario:
    """Special item models in the hotbar, hands, inventory, frames and stands; a name tag and
    formatted world text. Facing south, +x is screen left; the join toasts cover the right."""
    s = Scenario("flat_items_text", JOIN + 120)
    banner = 'minecraft:red_banner[banner_patterns=[{pattern:"minecraft:cross",color:"blue"}]]'
    specials = [
        "minecraft:shield",
        banner,
        "minecraft:red_bed",
        "minecraft:chest",
        "minecraft:player_head",
        "minecraft:trident",
        "minecraft:conduit",
        "minecraft:decorated_pot",
        "minecraft:purple_shulker_box",
    ]
    text = (
        '[{text:"Bold ",bold:1b,color:"red"},{text:"Ital ",italic:1b,color:"aqua"},'
        '{text:"Under",underlined:1b},{text:" Strike",strikethrough:1b},{text:" obf",obfuscated:1b}]'
    )
    s.server(
        JOIN - 50,
        "gamerule advance_time false",
        "time set 6000",
        "weather clear",
        f"tp @a 0.5 {FLOOR} 0.5 0 0",
        f"fill -6 {FLOOR} 9 8 {FLOOR + 4} 9 minecraft:light_gray_concrete",
        *(f"give @a {item}" for item in specials),
        f"summon minecraft:pig 3.5 {FLOOR} 5 "
        '{NoAI:1b,Silent:1b,CustomNameVisible:1b,CustomName:{text:"Named Pig",color:"gold",bold:true}}',
        f"summon text_display 0.5 {FLOOR + 2.4} 7 {{billboard:\"center\",text:{text}}}",
        f"setblock 2 {FLOOR} 3 minecraft:oak_sign[rotation=8]"
        '{front_text:{has_glowing_text:1b,color:"light_blue",'
        'messages:[{text:"Glow"},{text:"Sign",bold:true},"",""]}}',
        f"summon minecraft:armor_stand 1.5 {FLOOR} 6.5 "
        '{ShowArms:1b,Rotation:[180f,0f],equipment:{mainhand:{id:"minecraft:shield"},'
        f'head:{{id:"{banner.split("[")[0]}"}}}}}}',
        f"summon minecraft:glow_item_frame 4.5 {FLOOR + 2} 8.5 "
        '{Facing:2b,Item:{id:"minecraft:chest",count:1}}',
        f"summon minecraft:item_frame -0.5 {FLOOR + 2} 8.5 "
        '{Facing:2b,Item:{id:"minecraft:player_head",count:1}}',
        f'summon minecraft:item 0.5 {FLOOR + 1} 3 {{Item:{{id:"minecraft:red_bed",count:1}}}}',
    )
    t = JOIN
    s.shot(t, "world").shot(t + 20, "world_later")
    s.key(t + 25, "hotbar2").shot(t + 30, "hand_banner")
    s.key(t + 35, "hotbar6").shot(t + 40, "hand_trident")
    s.key(t + 45, "hotbar3").key(t + 48, "f5").shot(t + 55, "third_person_bed")
    s.key(t + 58, "f5").key(t + 59, "f5")
    s.key(t + 65, "inventory").shot(t + 70, "inventory_specials").key(t + 75, "escape")
    return s


def flat_block_states() -> Scenario:
    """Rotated variants, multipart connections and flat (zero-thickness) block models, seen from
    the north and from the east. Facing south, +x is screen left."""
    s = Scenario("flat_block_states", JOIN + 40)
    blocks = [
        (3, 3, "furnace[facing=east]"),
        (1, 3, "oak_stairs[facing=north]"),
        (-1, 3, "piston[facing=up]"),
        (-3, 3, "red_mushroom_block"),
        (3, 6, "rail[shape=east_west]"),
        (1, 6, "wheat[age=7]"),
        (-1, 7, "stone"),
        (-1, 6, "ladder[facing=north]"),
        (-3, 6, "oak_fence"),
        (-2, 6, "oak_planks"),
        (2, 1, "leaf_litter[segment_amount=4]"),
        (-1, 1, "pink_petals[flower_amount=4]"),
    ]
    s.server(
        JOIN - 50,
        "gamerule advance_time false",
        "time set 6000",
        "weather clear",
        f"tp @a 0.5 {FLOOR} -1.5 0 30",
        *(f"setblock {x} {FLOOR} {z} minecraft:{block}" for x, z, block in blocks),
    )
    s.shot(JOIN, "north")
    s.server(JOIN + 10, f"tp @a 6.5 {FLOOR} 4.5 90 25").shot(JOIN + 25, "east")
    return s


def flat_block_shapes() -> Scenario:
    """Rotated, inverted and multipart-combined models (fire, bars, bamboo, spawners) next to
    plain crosses as a control. Facing south, +x is screen left."""
    s = Scenario("flat_block_shapes", JOIN + 150)
    below = [
        (4, 2, "netherrack"),
        (2, 2, "soul_sand"),
        (0, 2, "grass_block"),
        (-1, 2, "grass_block"),
        (-2, 2, "sand"),
        (-2, 3, "water"),
        *((x, 5, "grass_block") for x in (3, 4, 5)),
    ]
    blocks = [
        (4, 2, "fire"),
        (2, 2, "soul_fire"),
        (0, 2, "poppy"),
        (-1, 2, "short_grass"),
        (-2, 2, "sugar_cane"),
        (-4, 2, "iron_bars"),
        (5, 5, "bamboo[age=0,leaves=none,stage=0]"),
        (4, 5, "bamboo[age=0,leaves=small,stage=0]"),
        (3, 5, "bamboo[age=1,leaves=large,stage=0]"),
        (1, 5, "iron_bars[west=true]"),
        (0, 5, "iron_bars[east=true,west=true]"),
        (-1, 5, "iron_bars[east=true]"),
        (-3, 5, "copper_bars"),
        (-5, 5, "copper_bars[west=true]"),
        (-6, 5, "copper_bars[east=true]"),
        (3, 8, "spawner"),
        (0, 8, "trial_spawner"),
        (-3, 8, "vault[facing=north]"),
    ]
    s.server(
        JOIN - 50,
        "gamerule advance_time false",
        "gamerule fire_spread_radius_around_player 0",
        "time set 6000",
        "weather clear",
        f"tp @a 0.5 {FLOOR} -3.5 0 25",
        *(f"setblock {x} {FLOOR - 1} {z} minecraft:{block}" for x, z, block in below),
        *(f"setblock {x} {FLOOR} {z} minecraft:{block}" for x, z, block in blocks),
    )
    # After the join toasts have gone.
    s.shot(JOIN + 110, "north")
    s.server(JOIN + 120, f"tp @a 10.5 {FLOOR} 5.5 90 25").shot(JOIN + 140, "east")
    return s


def terrain_tour() -> Scenario:
    s = Scenario("terrain_tour", 360, world=TERRAIN_TEMPLATE, render_distance=8)
    stops = [
        (0, 120, 0, 0, 30),
        (64, 110, 64, 135, 20),
        (-96, 130, 40, 270, 35),
        (30, 100, -120, 45, 10),
    ]
    for index, (x, y, z, yaw, pitch) in enumerate(stops):
        step = 20 + index * 80
        s.server(step, f"tp @a {x} {y} {z} {yaw} {pitch}")
        s.shot(step + 70, f"stop{index}")
    return s


SCENARIOS: dict[str, Callable[[], Scenario]] = {
    f().name: f
    for f in (
        flat_idle,
        flat_move,
        flat_ui,
        flat_entities,
        flat_blocks,
        flat_environment,
        flat_hud,
        flat_containers,
        flat_bestiary,
        flat_particles,
        flat_menus,
        flat_screens,
        flat_items_text,
        flat_block_states,
        flat_block_shapes,
        terrain_tour,
    )
}


# --- the lockstep loop ---------------------------------------------------------


@dataclass
class Session:
    scenario: Scenario
    client: Client
    link: Link
    out: Path
    #: Recording only waits for the simulation and keeps server pace: the
    #: real server times out a client whose windows stall on render work.
    recording: bool = False
    #: Wall seconds to sample the frame rate at each screenshot (0: off).
    fps_seconds: float = 0.0
    delivered: int = 0
    barrier_seconds: list[float] = field(default_factory=list)
    fps: dict[str, float] = field(default_factory=dict)
    frame_times_ms: list[float] = field(default_factory=list)
    last_frames_rendered: int | None = None
    mesh_counters: dict[str, Any] = field(default_factory=dict)
    #: (window, audio log lines written by then), taken at quiescence: the
    #: client logs audio as its step applies it, so lines settle with it.
    audio_marks: list[tuple[int, int]] = field(default_factory=list)

    def barrier(self, window: int) -> None:
        start = time.monotonic()
        previous = None
        next_report = 5.0
        while True:
            self.client.check_alive()
            status = self.client.call("sim.lockstep_status")
            ready = (
                status["simulation_quiescent" if self.recording else "quiescent"]
                and status["net_frames_read"] == self.delivered
                and status["net_frames_written"] == self.link.serverbound_count()
            )
            raw_frames = status.pop("frames_rendered", None)
            if isinstance(raw_frames, int) and not isinstance(raw_frames, bool):
                self.last_frames_rendered = raw_frames
            if ready and status == previous:
                break
            previous = status if ready else None
            waited = time.monotonic() - start
            if waited > next_report:
                print(
                    f"  window {window}: waiting {waited:.0f}s delivered={self.delivered} {status}",
                    file=sys.stderr,
                    flush=True,
                )
                next_report += 5
            if waited > BARRIER_TIMEOUT:
                raise ReplayError(
                    f"window {window}: no quiescence after {BARRIER_TIMEOUT}s;"
                    f" delivered {self.delivered}, sb {self.link.serverbound_count()}, {status}"
                )
            time.sleep(POLL)
        self.barrier_seconds.append(time.monotonic() - start)

    def run(
        self, frames_for: Callable[[int], list[bytes]], on_server: Callable[[str], None]
    ) -> None:
        self.client.call("input.focus", {"focused": True})
        if self.client.transport == "mcp":
            tools, _ = self.client._mcp_request("tools/list")
            (self.out / "mcp-tools.json").write_text(json.dumps(tools, indent=2) + "\n")
        shots = self.out / "shots"
        shots.mkdir(exist_ok=True)
        window_started = time.monotonic()
        for window in range(self.scenario.steps):
            if self.recording:
                time.sleep(max(0.0, window_started + 0.05 - time.monotonic()))
                window_started = time.monotonic()
            frames = frames_for(window)
            self.link.deliver(frames)
            self.delivered += len(frames)
            self.barrier(window)
            actions = self.scenario.actions.get(window, [])
            for kind, value in actions:
                if kind == "server":
                    on_server(value)
                elif kind == "control":
                    result = self.client.call(value[0], value[1])
                    with (self.out / "actions.jsonl").open("a") as handle:
                        handle.write(json.dumps({
                            "window": window, "method": value[0], "params": value[1],
                            "result": result,
                        }) + "\n")
                elif kind == "shot" and (not self.recording or self.client.transport == "mcp"):
                    self.screenshot(shots / f"{value}.png", window)
                    if self.fps_seconds:
                        self.fps[value] = self.sample_fps()
            if actions:
                self.barrier(window)
            before_frames = self.last_frames_rendered
            step_started = time.monotonic()
            self.client.call("sim.step", {"ticks": 1})
            self.barrier(window)
            step_ms = (time.monotonic() - step_started) * 1000.0
            after_frames = self.last_frames_rendered
            delta_frames = (
                after_frames - before_frames
                if before_frames is not None
                and after_frames is not None
                and after_frames > before_frames
                else 1
            )
            self.frame_times_ms.append(step_ms / delta_frames)
            if not self.recording:
                self.audio_marks.append((window, count_lines(self.out / "audio.jsonl")))
        counters = self.client.call("renderer.counters")
        if isinstance(counters, dict):
            self.mesh_counters = counters

    def sample_fps(self) -> float:
        first = self.client.call("sim.lockstep_status")["frames_rendered"]
        started = time.monotonic()
        time.sleep(self.fps_seconds)
        last = self.client.call("sim.lockstep_status")["frames_rendered"]
        return round((last - first) / (time.monotonic() - started), 1)

    def screenshot(self, path: Path, window: int) -> None:
        path.unlink(missing_ok=True)
        result = self.client.call("renderer.screenshot", {"path": str(path)})
        if self.client.transport == "mcp":
            self.completed_screenshot(path, result)
            self.barrier(window)
            return
        deadline = time.monotonic() + 60
        size = -1
        while not path.exists() or path.stat().st_size != size or path.stat().st_size == 0:
            size = path.stat().st_size if path.exists() else -1
            if time.monotonic() > deadline:
                raise ReplayError(f"window {window}: screenshot {path.name} never landed")
            self.client.check_alive()
            time.sleep(0.05)
        self.barrier(window)

    def completed_screenshot(self, path: Path, result: Any) -> None:
        if not isinstance(result, dict) or result.get("completed") is not True:
            raise ReplayError(f"MCP screenshot {path.name}: missing completed receipt")
        if (
            not isinstance(result.get("path"), str)
            or Path(result["path"]).resolve() != path.resolve()
        ):
            raise ReplayError(f"MCP screenshot {path.name}: receipt path does not match request")
        if result.get("mime_type") != "image/png" or any(
            type(result.get(key)) is not int or result[key] <= 0 for key in ("width", "height")
        ):
            raise ReplayError(f"MCP screenshot {path.name}: invalid image metadata")
        content = self.client.last_content
        images = [
            item for item in content if isinstance(item, dict) and item.get("type") == "image"
        ] if isinstance(content, list) else []
        if len(images) != 1 or images[0].get("mimeType") != result["mime_type"]:
            raise ReplayError(f"MCP screenshot {path.name}: missing matching image content")
        try:
            data = base64.b64decode(images[0]["data"], validate=True)
            if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
                raise ValueError("image content is not PNG")
            if struct.unpack_from(">II", data, 16) != (result["width"], result["height"]):
                raise ValueError("image dimensions do not match receipt")
            if path.read_bytes() != data:
                raise ValueError("image content does not match requested file")
        except (OSError, ValueError, TypeError, KeyError) as error:
            raise ReplayError(f"MCP screenshot {path.name}: {error}") from error


def count_lines(path: Path) -> int:
    try:
        return path.read_bytes().count(b"\n")
    except FileNotFoundError:
        return 0


def cpu_seconds(pid: int) -> float:
    return sample_process_metrics(pid).cpu_seconds


def peak_rss(pid: int) -> int:
    return sample_process_metrics(pid).peak_rss_kib


def git_rev() -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=Path(__file__).parent
    ).stdout.strip()


def record(tools: Tools, scenario: Scenario) -> Path:
    capture = CACHE / scenario.name / "capture"
    shutil.rmtree(capture, ignore_errors=True)
    capture.mkdir(parents=True)
    write_plan(capture, scenario)
    paths = Paths(
        artifact_dir=capture,
        server_bin=tools.bin("bbb-server"),
        client_bin=tools.bin("bbb-client"),
        admin_bin=tools.bin("bbb-admin"),
        vanilla_root=tools.vanilla_root or default_vanilla_root(),
    )
    world = None
    if scenario.world is not None:
        world = capture / "world"
        shutil.copytree(scenario.world, world)
    server = ServerProcess(
        paths=paths,
        name="server",
        game_port=free_port(),
        management_port=free_port(),
        admin_dir=paths.child("admin"),
        world_dir=world,
        loopback_scope="command",
    )
    weston = display_session(capture)
    windows: dict[int, list[bytes]] = {}
    try:
        server.start()
        server.wait_ready()
        link = Link(server.game_port)
        client = Client(tools, capture, weston, link.port, scenario)
        try:
            client.wait_control()
            link.accept()
            session = Session(scenario, client, link, capture, recording=True)

            def frames_for(window: int) -> list[bytes]:
                frames = link.take_held()
                if frames:
                    windows[window] = frames
                return frames

            def on_server(command: str) -> None:
                answer = server.management(
                    "call", "command.execute", "--arguments-json", json.dumps({"command": command})
                )
                with (capture / "commands.jsonl").open("a") as handle:
                    handle.write(json.dumps({"command": command, "answer": answer}) + "\n")

            session.run(frames_for, on_server)
        finally:
            client.quit()
            link.close()
    finally:
        weston.close()
        server.terminate()
    write_capture(capture / "clientbound.bin", windows)
    first = [frame for window in sorted(windows) for frame in windows[window]]
    meta = {
        "scenario": scenario.name,
        "steps": scenario.steps,
        "recorded_at": git_rev(),
        "compression": detect_compression(first),
        "frames": len(first),
        "bytes": sum(map(len, first)),
    }
    (capture / "session.json").write_text(json.dumps(meta, indent=2) + "\n")
    shutil.rmtree(capture / "game", ignore_errors=True)
    if world is not None:
        shutil.rmtree(world, ignore_errors=True)
    return capture


def percentile_sorted(sorted_samples: list[float], pct: float) -> float:
    pos = (len(sorted_samples) - 1) * (pct / 100.0)
    lo = math.floor(pos)
    hi = math.ceil(pos)
    if lo == hi:
        return sorted_samples[lo]
    weight = pos - lo
    return sorted_samples[lo] * (1.0 - weight) + sorted_samples[hi] * weight


def compute_frame_time_stats(samples_ms: list[float]) -> dict[str, Any]:
    clean = sorted(float(v) for v in samples_ms if math.isfinite(v) and v >= 0.0)
    if not clean:
        return {
            "sample_count": 0,
            "min_ms": None,
            "mean_ms": None,
            "p50_ms": None,
            "p95_ms": None,
            "p99_ms": None,
            "max_ms": None,
        }
    return {
        "sample_count": len(clean),
        "min_ms": round(clean[0], 3),
        "mean_ms": round(sum(clean) / len(clean), 3),
        "p50_ms": round(percentile_sorted(clean, 50.0), 3),
        "p95_ms": round(percentile_sorted(clean, 95.0), 3),
        "p99_ms": round(percentile_sorted(clean, 99.0), 3),
        "max_ms": round(clean[-1], 3),
    }


def summarize_mesh_worker(counters: dict[str, Any] | None) -> dict[str, int]:
    c = counters or {}
    return {
        "build_nanos": int(c.get("terrain_mesh_recent_build_nanos") or 0),
        "peak_build_nanos": int(c.get("terrain_mesh_peak_build_nanos") or 0),
        "queue_full": int(c.get("terrain_mesh_queue_full") or 0),
        "jobs_queued": int(c.get("terrain_mesh_jobs_queued") or 0),
        "jobs_completed": int(c.get("terrain_mesh_jobs_completed") or 0),
        "jobs_stale": int(c.get("terrain_mesh_jobs_stale") or 0),
        "sections_queued": int(c.get("terrain_mesh_sections_queued") or 0),
        "sections_completed": int(c.get("terrain_mesh_sections_completed") or 0),
        "sections_stale": int(c.get("terrain_mesh_sections_stale") or 0),
        "queue_depth": int(c.get("terrain_mesh_queue_depth") or 0),
        "peak_queue_depth": int(c.get("terrain_mesh_peak_queue_depth") or 0),
    }


def replay(tools: Tools, scenario: Scenario, out: Path, fps_seconds: float = 0.0) -> dict[str, Any]:
    out = out.resolve()
    capture = CACHE / scenario.name / "capture"
    scenario = read_plan(capture, scenario.name) or scenario
    meta = json.loads((capture / "session.json").read_text())
    windows = read_capture(capture / "clientbound.bin")
    shutil.rmtree(out, ignore_errors=True)
    out.mkdir(parents=True)
    write_plan(out, scenario)
    weston = display_session(out)
    link = Link(None)
    try:
        client = Client(tools, out, weston, link.port, scenario)
        started = time.monotonic()
        try:
            client.wait_control()
            link.accept()
            session = Session(scenario, client, link, out, fps_seconds=fps_seconds)
            session.run(lambda window: windows.get(window, []), lambda _command: None)
            elapsed = time.monotonic() - started
            peak_rss_kib = peak_rss(client.process.pid)
            cpu = cpu_seconds(client.process.pid)
        finally:
            client.quit()
            link.close()
    finally:
        weston.close()
    log = SessionLog(meta["compression"])
    with link.lock:
        frames = list(link.serverbound)
    # Window tags are informational; ClientTickEnd marks tick structure.
    for frame in frames:
        log.add(frame, -1)
    with (out / "serverbound.jsonl").open("w") as handle:
        for record_ in log.records:
            handle.write(json.dumps(record_) + "\n")
    audio = (out / "audio.jsonl").read_text().splitlines() if (out / "audio.jsonl").exists() else []
    with (out / "audio-windows.jsonl").open("w") as handle:
        start = 0
        for window, end in session.audio_marks:
            for line in audio[start:end]:
                handle.write(json.dumps({"window": window, **json.loads(line)}) + "\n")
            start = max(start, end)
        for line in audio[start:]:
            handle.write(json.dumps({"window": "after", **json.loads(line)}) + "\n")
    files = {}
    for path in sorted((out / "game").rglob("*")):
        if path.is_file() and "logs" not in path.relative_to(out / "game").parts:
            files[path.relative_to(out / "game").as_posix()] = hashlib.sha256(
                path.read_bytes()
            ).hexdigest()
    (out / "files.json").write_text(json.dumps(files, indent=2) + "\n")
    timing = {
        "elapsed_seconds": round(elapsed, 3),
        "barrier_seconds_total": round(sum(session.barrier_seconds), 3),
        "barrier_seconds_max": round(max(session.barrier_seconds, default=0), 3),
        "windows": scenario.steps,
        "peak_rss_mib": round(peak_rss_kib / 1024, 1),
        # Client user+system time: steadier than wall time on a shared host.
        "cpu_seconds": round(cpu, 2),
        "fps": session.fps,
        "frame_time_ms": compute_frame_time_stats(session.frame_times_ms),
        "mesh_worker": summarize_mesh_worker(session.mesh_counters),
    }
    (out / "timing.json").write_text(json.dumps(timing, indent=2) + "\n")
    return timing


# --- compare -------------------------------------------------------------------


def read_jsonl(path: Path) -> list[Any]:
    if not path.is_file():
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def first_mismatch(left: list[Any], right: list[Any]) -> dict[str, Any] | None:
    for index, (a, b) in enumerate(zip(left, right)):
        if a != b:
            return {"index": index, "left": a, "right": b}
    if len(left) != len(right):
        return {"index": min(len(left), len(right)), "left_len": len(left), "right_len": len(right)}
    return None


def compare_scenario(
    tools: Tools, scenario: Scenario, left: Path, right: Path, diffs: Path
) -> dict[str, Any]:
    strip = lambda records: [{k: v for k, v in r.items() if k != "window"} for r in records]  # noqa: E731
    left_scenario = read_plan(left, scenario.name) or scenario
    right_scenario = read_plan(right, scenario.name) or scenario
    left_plan, right_plan = scenario_plan(left_scenario), scenario_plan(right_scenario)
    report: dict[str, Any] = {"plan": None}
    if left_plan != right_plan:
        return {
            "plan": {"left": left_plan, "right": right_plan},
            "serverbound": None, "audio": None, "files": None, "shots": None,
        }
    report["serverbound"] = first_mismatch(
        strip(read_jsonl(left / "serverbound.jsonl")),
        strip(read_jsonl(right / "serverbound.jsonl")),
    )
    report["audio"] = first_mismatch(
        read_jsonl(left / "audio-windows.jsonl"), read_jsonl(right / "audio-windows.jsonl")
    )
    left_files = json.loads((left / "files.json").read_text())
    right_files = json.loads((right / "files.json").read_text())
    report["files"] = (
        sorted(
            name
            for name in set(left_files) | set(right_files)
            if left_files.get(name) != right_files.get(name)
        )
        or None
    )
    shots = sorted(
        {p.name for p in (left / "shots").glob("*.png")}
        | {p.name for p in (right / "shots").glob("*.png")}
    )
    diffs.mkdir(parents=True, exist_ok=True)
    jobs, missing = [], []
    for name in shots:
        a, b = left / "shots" / name, right / "shots" / name
        if not a.exists() or not b.exists():
            missing.append(name)
        elif a.read_bytes() != b.read_bytes():
            masks = left_scenario.masks.get(name.removesuffix(".png"), [])
            jobs.append(
                {"left": str(a), "right": str(b), "diff": str(diffs / name), "masks": masks}
            )
    pixels = {}
    if jobs:
        output = subprocess.run(
            [str(tools.bin("bbb-replay-diff"))],
            input=json.dumps(jobs),
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        for line in output.splitlines():
            entry = json.loads(line)
            pixels[Path(entry.pop("left")).name] = entry
    report["shots"] = {"missing": missing, "differing": pixels} if missing or pixels else None
    return report


def passes(report: dict[str, Any], delta: int, max_pixels: int) -> bool:
    """Packets, audio and files must match exactly; a screenshot may differ in
    at most `max_pixels` pixels by more than `delta` per channel. Client
    particles draw from RNG streams no replay pins, so a few stray particle
    pixels between identical builds are expected."""
    if report.get("plan") or report["serverbound"] or report["audio"] or report["files"]:
        return False
    shots = report["shots"]
    if not shots:
        return True
    if shots["missing"]:
        return False
    return all(
        "size_mismatch" not in entry and entry["over"][str(delta)] <= max_pixels
        for entry in shots["differing"].values()
    )


# --- CLI -------------------------------------------------------------------------


def selected(names: list[str], *, captures: bool = False) -> list[Scenario]:
    if not names or names == ["all"]:
        return [factory() for factory in SCENARIOS.values()]
    scenarios = []
    for name in names:
        if Path(name).name != name or name in (".", ".."):
            raise ReplayError(f"invalid scenario name {name!r}")
        if name in SCENARIOS:
            scenarios.append(SCENARIOS[name]())
        else:
            saved = read_plan(CACHE / name / "capture", name) if captures else None
            if saved is None:
                raise ReplayError(f"unknown scenario {name!r}; known: {sorted(SCENARIOS)}")
            scenarios.append(saved)
    return scenarios


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--target",
        type=Path,
        default=default_target_debug_dir(),
        help="directory holding bbb-client, bbb-server, bbb-admin and bbb-replay-diff",
    )
    parser.add_argument("--transport", choices=("control", "mcp"), default="control")
    parser.add_argument(
        "--software-gpu",
        action="store_true",
        default=os.environ.get("BBB_SOFTWARE_GPU") == "1",
        help="request fallback/software wgpu adapter (BBB_SOFTWARE_GPU=1)",
    )
    parser.add_argument("--vanilla-root", type=Path, default=None)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("list")
    rec = sub.add_parser("record")
    rec.add_argument("scenarios", nargs="*")
    rep = sub.add_parser("replay")
    rep.add_argument("--out", type=Path, required=True)
    rep.add_argument(
        "--fps-seconds", type=float, default=0.0, help="sample frame rate at each screenshot"
    )
    rep.add_argument("scenarios", nargs="*")
    cmp_ = sub.add_parser("compare")
    cmp_.add_argument("left", type=Path)
    cmp_.add_argument("right", type=Path)
    cmp_.add_argument(
        "--delta", type=int, default=8, choices=[0, 2, 8, 32], help="per-channel change that counts"
    )
    cmp_.add_argument(
        "--max-pixels", type=int, default=64, help="changed pixels a screenshot may have"
    )
    cmp_.add_argument("scenarios", nargs="*")
    args = parser.parse_args(argv)
    tools = Tools(
        args.target,
        args.transport,
        software_gpu=args.software_gpu,
        vanilla_root=args.vanilla_root,
    )

    if args.command == "list":
        for name, factory in SCENARIOS.items():
            scenario = factory()
            captured = (CACHE / name / "capture" / "session.json").is_file()
            print(f"{name:20} {scenario.steps:4} windows  {'captured' if captured else '-'}")
        return 0
    if args.command == "record":
        for scenario in selected(args.scenarios):
            started = time.monotonic()
            capture = record(tools, scenario)
            print(
                f"recorded {scenario.name} in {time.monotonic() - started:.1f}s -> {capture}",
                flush=True,
            )
        return 0
    if args.command == "replay":
        for scenario in selected(args.scenarios, captures=True):
            timing = replay(tools, scenario, args.out / scenario.name, args.fps_seconds)
            print(f"replayed {scenario.name}: {json.dumps(timing)}", flush=True)
        return 0
    failed = []
    for scenario in selected(args.scenarios, captures=True):
        left, right = args.left / scenario.name, args.right / scenario.name
        if not left.is_dir() or not right.is_dir():
            print(f"{scenario.name}: missing output", flush=True)
            failed.append(scenario.name)
            continue
        report = compare_scenario(tools, scenario, left, right, args.right / scenario.name / "diff")
        ok = passes(report, args.delta, args.max_pixels)
        exact = passes(report, 0, 0)
        print(
            f"{scenario.name}: {'same' if exact else 'within tolerance' if ok else 'DIFFERENT'}",
            flush=True,
        )
        if not exact:
            print(json.dumps(report, indent=2)[:4000], flush=True)
        if not ok:
            failed.append(scenario.name)
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except ReplayError as error:
        print(f"client_replay: {error}", file=sys.stderr)
        sys.exit(1)
