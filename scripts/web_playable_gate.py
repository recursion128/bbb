#!/usr/bin/env python3
"""End-to-end Chromium WebGPU playable gate for the BBB WASM client + bbb-web-host.

Starts a real `bbb-server` and `bbb-web-host` (with both `--control` and `--mcp-bind`),
opens the WASM + WebGPU client in local Chromium, verifies COOP/COEP + SharedArrayBuffer
+ WebGPU initialization via Chrome DevTools Protocol, waits for full Play join and
Rayon Web Worker chunk meshing/upload, drives camera look & player movement through
the reverse control tunnel, verifies the Streamable HTTP MCP endpoint (`/mcp`),
and captures a real async WebGPU PNG screenshot from the browser canvas.
"""

from __future__ import annotations

import argparse
import base64
import glob
import gzip
import hashlib
import http.client
import http.server
import io
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
import urllib.request
import zipfile
import zlib
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from bbb_tools.execution.process import free_port, terminate_process_tree  # noqa: E402
from bbb_tools.python_runner import (  # noqa: E402
    default_target_dir as shared_default_target_dir,
)
from bbb_tools.python_runner import (  # noqa: E402
    default_target_root,
    default_web_cache_dir,
    default_web_out_dir,
)
from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

MIB_BYTES = 1024.0 * 1024.0


class WebGateError(RuntimeError):
    """Actionable failure in the end-to-end Web playable gate."""


def parse_positive_budget(raw: str) -> float:
    try:
        value = float(raw)
    except ValueError as err:
        raise argparse.ArgumentTypeError(f"invalid float {raw!r}") from err
    if not math.isfinite(value) or value <= 0.0:
        raise argparse.ArgumentTypeError(f"must be a positive finite number, got {raw!r}")
    return value


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


def probe_gateway_payloads(http_port: int) -> tuple[str, dict[str, Any]]:
    wasm_gzip_bytes: int | None = None
    wasm_req = urllib.request.Request(
        f"http://127.0.0.1:{http_port}/pkg/bbb_client_bg.wasm",
        headers={"Accept-Encoding": "gzip"},
    )
    with urllib.request.urlopen(wasm_req, timeout=15.0) as resp:
        body = resp.read()
        if resp.headers.get("Content-Encoding", "").lower() == "gzip":
            wasm_gzip_bytes = len(body)
            body = gzip.decompress(body)
    wasm_raw_bytes = len(body)
    wasm_sha256 = hashlib.sha256(body).hexdigest()

    bundle_url = f"http://127.0.0.1:{http_port}/assets/core-26.1.zip"
    with urllib.request.urlopen(bundle_url, timeout=30.0) as resp:
        core_bundle_bytes = len(resp.read())

    return wasm_sha256, {
        "wasm_raw_bytes": wasm_raw_bytes,
        "wasm_raw_mib": round(wasm_raw_bytes / MIB_BYTES, 3),
        "wasm_gzip_bytes": wasm_gzip_bytes,
        "wasm_gzip_mib": (
            round(wasm_gzip_bytes / MIB_BYTES, 3) if wasm_gzip_bytes is not None else None
        ),
        "core_bundle_bytes": core_bundle_bytes,
        "core_bundle_mib": round(core_bundle_bytes / MIB_BYTES, 3),
    }


def build_perf_telemetry(
    raw_sampler: dict[str, Any] | None,
    payload_sizes: dict[str, Any],
    *,
    first_playable_ms: float,
    max_first_playable_ms: float | None = None,
    max_wasm_mib: float | None = None,
    max_frame_p95_ms: float | None = None,
) -> dict[str, Any]:
    sampler = raw_sampler or {}
    raw_frames = sampler.get("frameTimesMs") or []
    frame_samples = [float(x) for x in raw_frames if isinstance(x, int | float)]
    initial_bytes = max(0, int(sampler.get("initialWasmBytes") or 0))
    final_bytes = max(0, int(sampler.get("finalWasmBytes") or 0))
    peak_bytes = max(initial_bytes, final_bytes, int(sampler.get("peakWasmBytes") or 0))
    return {
        "first_playable_ms": round(first_playable_ms, 3),
        "frame_times": compute_frame_time_stats(frame_samples),
        "wasm_memory": {
            "initial_bytes": initial_bytes,
            "peak_bytes": peak_bytes,
            "final_bytes": final_bytes,
            "initial_mib": round(initial_bytes / MIB_BYTES, 3),
            "peak_mib": round(peak_bytes / MIB_BYTES, 3),
            "final_mib": round(final_bytes / MIB_BYTES, 3),
        },
        "payloads": payload_sizes,
        "budgets": {
            "max_first_playable_ms": max_first_playable_ms,
            "max_wasm_mib": max_wasm_mib,
            "max_frame_p95_ms": max_frame_p95_ms,
        },
    }


def enforce_perf_budgets(perf: dict[str, Any]) -> None:
    budgets = perf["budgets"]
    max_first_playable_ms = budgets["max_first_playable_ms"]
    first_playable_ms = perf["first_playable_ms"]
    if max_first_playable_ms is not None and first_playable_ms > max_first_playable_ms:
        raise WebGateError(
            f"First playable observation {first_playable_ms:.3f} ms exceeded "
            f"--max-first-playable-ms={max_first_playable_ms:.3f}"
        )

    max_wasm_mib = budgets["max_wasm_mib"]
    if max_wasm_mib is not None:
        peak_bytes = perf["wasm_memory"]["peak_bytes"]
        if peak_bytes == 0:
            raise WebGateError(
                "WASM linear memory watermark unavailable while --max-wasm-mib is set"
            )
        peak_mib = peak_bytes / MIB_BYTES
        if peak_mib > max_wasm_mib:
            raise WebGateError(
                f"WASM linear memory watermark {peak_mib:.3f} MiB exceeded "
                f"--max-wasm-mib={max_wasm_mib:.3f}"
            )

    max_frame_p95_ms = budgets["max_frame_p95_ms"]
    if max_frame_p95_ms is not None:
        p95_ms = perf["frame_times"]["p95_ms"]
        if p95_ms is None:
            raise WebGateError(
                "No active-play rAF frame-time samples recorded while "
                "--max-frame-p95-ms is set"
            )
        if p95_ms > max_frame_p95_ms:
            raise WebGateError(
                f"Active-play frame time p95 {p95_ms:.3f} ms exceeded "
                f"--max-frame-p95-ms={max_frame_p95_ms:.3f}"
            )


SERVER_PACK_PROBE_VALUE = "WebServerPackAccepted"
SERVER_PACK_PROMPT = "Use the test server artwork?"


def build_required_server_pack_zip() -> tuple[bytes, str]:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr(
            "pack.mcmeta",
            json.dumps(
                {
                    "pack": {
                        "description": "BBB Web Required Server Pack",
                        "min_format": [84, 0],
                        "max_format": [84, 0],
                    }
                }
            ),
        )
        zf.writestr(
            "assets/minecraft/lang/zh_cn.json",
            json.dumps({"menu.multiplayer": SERVER_PACK_PROBE_VALUE}),
        )
    payload = buf.getvalue()
    return payload, hashlib.sha1(payload).hexdigest()


def start_loopback_pack_server(pack_bytes: bytes) -> tuple[http.server.ThreadingHTTPServer, str]:
    class PackHandler(http.server.BaseHTTPRequestHandler):
        def do_GET(self) -> None:  # noqa: N802
            if self.path != "/server-pack.zip":
                self.send_response(404)
                self.end_headers()
                return
            self.send_response(200)
            self.send_header("Content-Type", "application/zip")
            self.send_header("Content-Length", str(len(pack_bytes)))
            self.end_headers()
            self.wfile.write(pack_bytes)

        def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
            pass

    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), PackHandler)
    port = httpd.server_address[1]
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    return httpd, f"http://127.0.0.1:{port}/server-pack.zip"


def find_chrome_binary() -> str:
    env_bin = os.environ.get("BBB_CHROME_BIN")
    if env_bin and Path(env_bin).is_file():
        return env_bin
    for name in (
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "chrome",
        "msedge",
    ):
        found = shutil.which(name)
        if found:
            return found
    if sys.platform == "win32":
        win_roots = [
            os.environ.get("PROGRAMFILES"),
            os.environ.get("PROGRAMFILES(X86)"),
            os.environ.get("LOCALAPPDATA"),
        ]
        for root in win_roots:
            if not root:
                continue
            for rel in (
                "Google/Chrome/Application/chrome.exe",
                "Chromium/Application/chrome.exe",
                "Microsoft/Edge/Application/msedge.exe",
            ):
                cand = Path(root) / rel
                if cand.is_file():
                    return str(cand)
    elif sys.platform == "darwin":
        for app_bin in (
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ):
            if Path(app_bin).is_file():
                return app_bin
    cache_candidates = sorted(
        glob.glob(str(Path.home() / ".cache/puppeteer/chrome/**/chrome*"), recursive=True),
        reverse=True,
    )
    for cand in cache_candidates:
        if os.access(cand, os.X_OK) and Path(cand).is_file():
            return cand
    raise WebGateError(
        "No Chrome/Chromium/Edge binary found (set BBB_CHROME_BIN or install Chrome/Edge)"
    )


def default_vanilla_root() -> Path:
    return VanillaWorkspace.resolve("26.1").root


def offline_player_uuid(username: str) -> str:
    raw = bytearray(
        hashlib.md5(
            f"OfflinePlayer:{username}".encode(), usedforsecurity=False
        ).digest()
    )
    raw[6] = (raw[6] & 0x0F) | 0x30
    raw[8] = (raw[8] & 0x3F) | 0x80
    hx = raw.hex()
    return f"{hx[0:8]}-{hx[8:12]}-{hx[12:16]}-{hx[16:20]}-{hx[20:32]}"


class SimpleWsClient:
    """Minimal stdlib RFC 6455 client for Chrome DevTools Protocol."""

    def __init__(self, ws_url: str, timeout: float = 15.0) -> None:
        assert ws_url.startswith("ws://"), ws_url
        rest = ws_url[len("ws://") :]
        host_port, path = rest.split("/", 1)
        host, port_s = host_port.split(":")
        self.sock = socket.create_connection((host, int(port_s)), timeout=timeout)
        key = base64.b64encode(os.urandom(16)).decode("ascii")
        req = (
            f"GET /{path} HTTP/1.1\r\n"
            f"Host: {host_port}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(req.encode("ascii"))
        self.buf = bytearray()
        while b"\r\n\r\n" not in self.buf:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise WebGateError("CDP WebSocket closed before handshake")
            self.buf.extend(chunk)
        idx = self.buf.index(b"\r\n\r\n")
        head = bytes(self.buf[:idx]).decode("utf-8", errors="replace")
        del self.buf[: idx + 4]
        if not head.startswith("HTTP/1.1 101"):
            raise WebGateError(f"CDP WebSocket upgrade failed: {head.splitlines()[0]}")
        self.exceptions: list[str] = []
        self.console_errors: list[str] = []
        self._next_id = 1000

    def _read_exact(self, n: int) -> bytes:
        while len(self.buf) < n:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise WebGateError("CDP WebSocket EOF")
            self.buf.extend(chunk)
        out = bytes(self.buf[:n])
        del self.buf[:n]
        return out

    def send_json(self, obj: dict[str, Any]) -> None:
        payload = json.dumps(obj).encode("utf-8")
        mask = os.urandom(4)
        frame = bytearray([0x81])
        n = len(payload)
        if n <= 125:
            frame.append(0x80 | n)
        elif n <= 65535:
            frame.append(0x80 | 126)
            frame.extend(struct.pack("!H", n))
        else:
            frame.append(0x80 | 127)
            frame.extend(struct.pack("!Q", n))
        frame.extend(mask)
        frame.extend(b ^ mask[i & 3] for i, b in enumerate(payload))
        self.sock.sendall(frame)

    def recv_json(self) -> dict[str, Any]:
        while True:
            b0, b1 = self._read_exact(2)
            opcode = b0 & 0x0F
            masked = (b1 & 0x80) != 0
            length = b1 & 0x7F
            if length == 126:
                (length,) = struct.unpack("!H", self._read_exact(2))
            elif length == 127:
                (length,) = struct.unpack("!Q", self._read_exact(8))
            mask_key = self._read_exact(4) if masked else None
            payload = bytearray(self._read_exact(length))
            if mask_key:
                for i in range(length):
                    payload[i] ^= mask_key[i & 3]
            if opcode == 0x8:
                raise WebGateError("CDP WebSocket closed")
            if opcode == 0x9:
                pong = bytearray([0x8A, 0x80 | len(payload)])
                m = os.urandom(4)
                pong.extend(m)
                pong.extend(b ^ m[i & 3] for i, b in enumerate(payload))
                self.sock.sendall(pong)
                continue
            if opcode == 0x1:
                return json.loads(payload.decode("utf-8"))

    def call(self, msg_id: int | None, method: str, params: dict[str, Any] | None = None) -> tuple[dict[str, Any], list[str]]:
        if msg_id is None:
            self._next_id += 1
            msg_id = self._next_id
        self.send_json({"id": msg_id, "method": method, "params": params or {}})
        logs: list[str] = []
        while True:
            msg = self.recv_json()
            ev_method = msg.get("method")
            if ev_method == "Runtime.exceptionThrown":
                detail = json.dumps(msg.get("params", {}))[:600]
                self.exceptions.append(detail)
                logs.append(f"[exceptionThrown] {detail}")
            elif ev_method == "Runtime.consoleAPICalled":
                params_obj = msg.get("params", {})
                detail = json.dumps(params_obj)[:400]
                logs.append(detail)
                if params_obj.get("type") == "error":
                    self.console_errors.append(detail)
            if msg.get("id") == msg_id:
                return msg, logs

    def close(self) -> None:
        try:
            self.sock.close()
        except OSError:
            pass


def control_call(addr: tuple[str, int], method: str, params: dict[str, Any] | None = None, timeout: float = 20.0) -> dict[str, Any]:
    with socket.create_connection(addr, timeout=timeout) as sock:
        sock.settimeout(timeout)
        line = json.dumps({"method": method, "params": params or {}}) + "\n"
        sock.sendall(line.encode("utf-8"))
        buf = bytearray()
        while b"\n" not in buf:
            chunk = sock.recv(65536)
            if not chunk:
                break
            buf.extend(chunk)
    if not buf:
        raise WebGateError(f"empty control response for {method}")
    return json.loads(buf.decode("utf-8").splitlines()[0])


def mcp_call(mcp_port: int, payload: dict[str, Any], session_id: str | None = None) -> tuple[int, dict[str, str], dict[str, Any]]:
    conn = http.client.HTTPConnection("127.0.0.1", mcp_port, timeout=20.0)
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
    }
    if session_id:
        headers["Mcp-Session-Id"] = session_id
    conn.request("POST", "/mcp", body=json.dumps(payload).encode("utf-8"), headers=headers)
    resp = conn.getresponse()
    raw = resp.read().decode("utf-8")
    resp_headers = {k.lower(): v for k, v in resp.getheaders()}
    conn.close()
    body_json: dict[str, Any] = {}
    if raw.strip():
        if raw.lstrip().startswith("data:") or "\ndata:" in raw:
            for line in raw.splitlines():
                if line.startswith("data:"):
                    cand = line[5:].strip()
                    if cand:
                        body_json = json.loads(cand)
        else:
            body_json = json.loads(raw)
    return resp.status, resp_headers, body_json


def inspect_png(png_bytes: bytes) -> dict[str, Any]:
    if not png_bytes.startswith(b"\x89PNG\r\n\x1a\n"):
        raise WebGateError("captured screenshot does not start with PNG signature")
    pos = 8
    width = height = bit_depth = color_type = 0
    idat_chunks: list[bytes] = []
    while pos + 8 <= len(png_bytes):
        length = struct.unpack("!I", png_bytes[pos : pos + 4])[0]
        chunk_type = png_bytes[pos + 4 : pos + 8]
        data = png_bytes[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if chunk_type == b"IHDR":
            width, height, bit_depth, color_type = struct.unpack("!IIBB", data[:10])
        elif chunk_type == b"IDAT":
            idat_chunks.append(data)
        elif chunk_type == b"IEND":
            break
    if width < 320 or height < 200:
        raise WebGateError(f"unexpectedly small WebGPU screenshot: {width}x{height}")
    raw = zlib.decompress(b"".join(idat_chunks))
    bpp = 4 if color_type == 6 else (3 if color_type == 2 else 4)
    stride = 1 + width * bpp
    sampled_colors: set[tuple[int, int, int]] = set()
    # Sample unfiltered scanlines or Paeth-filtered variety heuristically; even better,
    # reconstruct Sub/Up/Average/Paeth on the first 64 evenly-spaced rows!
    prev_row = bytearray(width * bpp)
    step_y = max(1, height // 40)
    for y in range(height):
        row_start = y * stride
        if row_start + stride > len(raw):
            break
        filt = raw[row_start]
        cur = bytearray(raw[row_start + 1 : row_start + stride])
        if filt == 1:
            for i in range(bpp, len(cur)):
                cur[i] = (cur[i] + cur[i - bpp]) & 0xFF
        elif filt == 2:
            for i in range(len(cur)):
                cur[i] = (cur[i] + prev_row[i]) & 0xFF
        elif filt == 3:
            for i in range(len(cur)):
                left = cur[i - bpp] if i >= bpp else 0
                cur[i] = (cur[i] + ((left + prev_row[i]) >> 1)) & 0xFF
        elif filt == 4:
            for i in range(len(cur)):
                a = cur[i - bpp] if i >= bpp else 0
                b = prev_row[i]
                c = prev_row[i - bpp] if i >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                cur[i] = (cur[i] + pr) & 0xFF
        prev_row = cur
        if y % step_y == 0:
            step_x = max(1, width // 40)
            for x in range(0, width, step_x):
                off = x * bpp
                sampled_colors.add((cur[off], cur[off + 1], cur[off + 2]))
    if len(sampled_colors) < 8:
        raise WebGateError(
            f"WebGPU screenshot appears blank/uniform (only {len(sampled_colors)} distinct sampled RGB colors)"
        )
    return {
        "width": width,
        "height": height,
        "bit_depth": bit_depth,
        "distinct_sampled_rgb": len(sampled_colors),
        "bytes": len(png_bytes),
    }


def resolve_cargo_bin() -> str:
    explicit = os.environ.get("CARGO")
    if explicit and Path(explicit).is_file():
        return explicit
    found = shutil.which("cargo")
    if found:
        return found
    cargo_home = Path(os.environ.get("CARGO_HOME") or (Path.home() / ".cargo"))
    exe = "cargo.exe" if sys.platform == "win32" else "cargo"
    candidate = cargo_home / "bin" / exe
    if candidate.is_file():
        return str(candidate)
    raise WebGateError("cargo binary not found on PATH or in CARGO_HOME/bin")


def ensure_builds(wasm_dist: Path, skip_build: bool) -> tuple[Path, Path]:
    is_win = sys.platform == "win32"
    if skip_build:
        for name in ("bbb_client_bg.wasm", "bbb_client.js"):
            artifact = wasm_dist / "pkg" / name
            if not artifact.is_file():
                raise WebGateError(f"missing WASM bundle artifact {artifact} (--skip-build passed)")
    else:
        xtask_bin = os.environ.get("BBB_XTASK_BIN")
        if xtask_bin:
            build_web = build_native = [xtask_bin]
        else:
            cargo_bin = resolve_cargo_bin()
            build_web, build_native = [cargo_bin], [cargo_bin, "dev"]
        subprocess.run(
            [*build_web, "build-web", "--out-dir", str(wasm_dist)],
            cwd=REPO_ROOT,
            check=True,
        )
        subprocess.run(
            [*build_native, "build", "-p", "bbb-server", "-p", "bbb-web-host"],
            cwd=REPO_ROOT,
            check=True,
        )
    target_dir = shared_default_target_dir()
    exe_suffix = ".exe" if is_win else ""
    server_bin = target_dir / f"debug/bbb-server{exe_suffix}"
    host_bin = target_dir / f"debug/bbb-web-host{exe_suffix}"
    if not server_bin.is_file() or not host_bin.is_file():
        raise WebGateError(f"missing built binaries under {target_dir / 'debug'}")
    return server_bin, host_bin


def wait_for_server_ready(proc: subprocess.Popen[Any], log_path: Path, timeout: float = 90.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if log_path.is_file():
            for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
                if '"server.ready"' in line:
                    return
        if proc.poll() is not None:
            raise WebGateError(
                f"bbb-server exited early ({proc.returncode}):\n"
                + (log_path.read_text(encoding="utf-8", errors="replace")[-2000:] if log_path.is_file() else "")
            )
        time.sleep(0.1)
    raise WebGateError("timed out waiting for bbb-server ready")


def wait_for_http_ready(port: int, proc: subprocess.Popen[Any], log_path: Path, timeout: float = 180.0) -> dict[str, str]:
    deadline = time.monotonic() + timeout
    url = f"http://127.0.0.1:{port}/"
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise WebGateError(
                f"bbb-web-host exited early ({proc.returncode}):\n"
                + (log_path.read_text(encoding="utf-8", errors="replace")[-2000:] if log_path.is_file() else "")
            )
        try:
            with urllib.request.urlopen(url, timeout=3.0) as resp:
                if resp.status == 200:
                    return {k.lower(): v for k, v in resp.headers.items()}
        except Exception:
            time.sleep(0.15)
    raise WebGateError("timed out waiting for bbb-web-host HTTP listener")


def wait_for_devtools_ws(chrome_proc: subprocess.Popen[Any], stderr_path: Path, timeout: float = 20.0) -> int:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if stderr_path.is_file():
            text = stderr_path.read_text(encoding="utf-8", errors="replace")
            for line in text.splitlines():
                if "DevTools listening on ws://127.0.0.1:" in line:
                    rest = line.split("ws://127.0.0.1:", 1)[1]
                    return int(rest.split("/", 1)[0])
        if chrome_proc.poll() is not None:
            raise WebGateError(
                f"Chrome exited early ({chrome_proc.returncode}):\n"
                + (stderr_path.read_text(encoding="utf-8", errors="replace")[-2000:] if stderr_path.is_file() else "")
            )
        time.sleep(0.1)
    raise WebGateError("timed out waiting for Chrome DevTools endpoint")


def build_arg_parser(default_tmp: Path | None = None) -> argparse.ArgumentParser:
    tmp_dir = default_tmp if default_tmp is not None else default_target_root()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--wasm-dir",
        type=Path,
        default=default_web_out_dir(default_root=tmp_dir),
    )
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=default_web_cache_dir(default_root=tmp_dir),
    )
    parser.add_argument("--artifact-dir", type=Path, default=None)
    parser.add_argument("--vanilla-root", type=Path, default=None)
    parser.add_argument("--rebuild-web", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="reuse the supplied or default WASM bundle and existing native binaries",
    )
    parser.add_argument(
        "--headless",
        action="store_true",
        default=True,
        help="run Chrome with --headless=new (default)",
    )
    parser.add_argument(
        "--headed",
        dest="headless",
        action="store_false",
        help="open a visible Chrome window for local debugging",
    )
    parser.add_argument(
        "--software-gpu",
        action="store_true",
        default=os.environ.get("BBB_SOFTWARE_GPU") == "1",
        help="enable SwiftShader/software WebGPU adapter for GPU-less CI runners",
    )
    parser.add_argument(
        "--max-first-playable-ms",
        type=parse_positive_budget,
        default=None,
        help="optional upper bound on time from navigation to observed playable terrain in ms",
    )
    parser.add_argument(
        "--max-wasm-mib",
        type=parse_positive_budget,
        default=None,
        help="optional upper bound on peak WASM linear memory in MiB",
    )
    parser.add_argument(
        "--max-frame-p95-ms",
        type=parse_positive_budget,
        default=None,
        help="optional upper bound on active-play requestAnimationFrame p95 in ms",
    )
    parser.add_argument(
        "--level-seed",
        default="bbb-web-playable-26.1",
        help="deterministic level seed for reproducible spawn geometry and perf sampling",
    )
    return parser


def main() -> int:
    args = build_arg_parser().parse_args()

    chrome_bin = find_chrome_binary()
    vanilla_root = args.vanilla_root or default_vanilla_root()
    if not (
        (vanilla_root / "sources/26.1/version.json").is_file()
        or (vanilla_root / "sources/26.1/assets/minecraft").is_dir()
        or (vanilla_root / "downloads/26.1/client-26.1.jar").is_file()
        or (vanilla_root / "versions/26.1/26.1.jar").is_file()
    ):
        raise WebGateError(
            f"Vanilla 26.1 assets not found under {vanilla_root} (run `cargo dev vanilla prepare 26.1` or point at `.minecraft`)"
        )

    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO_ROOT, text=True).strip()
    dirty = bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=REPO_ROOT))
    wasm_dir = args.wasm_dir.resolve()
    server_bin, host_bin = ensure_builds(wasm_dir, args.skip_build)

    temp_ctx = tempfile.TemporaryDirectory(prefix="bbb-web-playable-")
    runtime_dir = Path(temp_ctx.name)
    work_dir = args.artifact_dir or runtime_dir
    work_dir.mkdir(parents=True, exist_ok=True)

    srv_port = free_port()
    srv_mcp_port = free_port()
    http_port = free_port()
    ctrl_port = free_port()
    mcp_port = free_port()

    srv_log = work_dir / "bbb-server.log"
    host_log = work_dir / "bbb-web-host.log"
    chrome_log = work_dir / "chrome.log"

    procs: list[subprocess.Popen[Any]] = []
    cdp: SimpleWsClient | None = None
    pack_httpd: http.server.ThreadingHTTPServer | None = None
    try:
        pack_bytes, pack_sha1 = build_required_server_pack_zip()
        pack_httpd, pack_url = start_loopback_pack_server(pack_bytes)
        admin_dir = runtime_dir / "admin"
        admin_dir.mkdir(parents=True, exist_ok=True)
        (admin_dir / "ops.json").write_text(
            json.dumps(
                [
                    {
                        "uuid": offline_player_uuid("WebPlayer"),
                        "name": "WebPlayer",
                        "level": 4,
                        "bypassesPlayerLimit": False,
                    }
                ]
            ),
            encoding="utf-8",
        )
        with srv_log.open("wb") as srv_out:
            srv_proc = subprocess.Popen(
                [
                    str(server_bin),
                    "--bind",
                    f"127.0.0.1:{srv_port}",
                    "--mcp-bind",
                    f"127.0.0.1:{srv_mcp_port}",
                    "--admin-dir",
                    str(admin_dir),
                    "--mcp-loopback-scope",
                    "manage",
                    "--resource-pack",
                    pack_url,
                    "--resource-pack-sha1",
                    pack_sha1,
                    "--require-resource-pack",
                    "--resource-pack-prompt",
                    json.dumps(SERVER_PACK_PROMPT),
                    "--gamemode",
                    "creative",
                    "--force-gamemode",
                    "--difficulty",
                    "peaceful",
                    "--view-distance",
                    "8",
                    "--simulation-distance",
                    "6",
                    "--pause-when-empty-seconds",
                    "0",
                    "--level-seed",
                    args.level_seed,
                    "--world",
                    str(runtime_dir / "world"),
                ],
                env={**os.environ, "RUST_LOG": "bbb_server=info"},
                stdin=subprocess.DEVNULL,
                stdout=srv_out,
                stderr=subprocess.STDOUT,
            )
            procs.append(srv_proc)
        wait_for_server_ready(srv_proc, srv_log)

        with host_log.open("wb") as host_out:
            host_proc = subprocess.Popen(
                [
                    str(host_bin),
                    "--bind",
                    f"127.0.0.1:{http_port}",
                    "--server",
                    f"127.0.0.1:{srv_port}",
                    "--vanilla-root",
                    str(vanilla_root),
                    "--wasm-dir",
                    str(wasm_dir),
                    "--cache-dir",
                    str(args.cache_dir),
                    "--control",
                    f"127.0.0.1:{ctrl_port}",
                    "--mcp-bind",
                    f"127.0.0.1:{mcp_port}",
                    "--allow-pack-loopback",
                ],
                stdin=subprocess.DEVNULL,
                stdout=host_out,
                stderr=subprocess.STDOUT,
            )
            procs.append(host_proc)

        headers = wait_for_http_ready(http_port, host_proc, host_log)
        assert headers.get("cross-origin-opener-policy") == "same-origin", headers
        assert headers.get("cross-origin-embedder-policy") == "require-corp", headers
        assert headers.get("cross-origin-resource-policy") == "same-origin", headers
        wasm_sha256, payload_sizes = probe_gateway_payloads(http_port)

        software_query = "&software_gpu=1" if args.software_gpu else ""
        server_url = (
            f"http://127.0.0.1:{http_port}/"
            f"?render_distance=8&fov=90&gui_scale=2&locale=zh_cn&server=localhost:{srv_port}{software_query}"
        )
        page_url = f"{server_url}&username=WebPlayer"
        chrome_args = [
            chrome_bin,
            *(["--headless=new"] if args.headless else []),
            "--no-sandbox",
            "--disable-gpu-sandbox",
            "--disable-background-timer-throttling",
            "--disable-backgrounding-occluded-windows",
            "--disable-renderer-backgrounding",
            "--disable-features=CalculateNativeWinOcclusion",
            "--enable-unsafe-webgpu",
            "--autoplay-policy=no-user-gesture-required",
            "--window-size=854,480",
            "--remote-debugging-port=0",
            f"--user-data-dir={runtime_dir / 'chrome-profile'}",
        ]
        if sys.platform.startswith("linux"):
            chrome_args.extend(
                [
                    "--disable-dev-shm-usage",
                    "--enable-features=Vulkan,UseSkiaRenderer",
                    "--use-angle=vulkan",
                ]
            )
        if args.software_gpu:
            if sys.platform.startswith("linux"):
                chrome_args.extend(
                    [
                        "--use-vulkan=swiftshader",
                        "--disable-vulkan-surface",
                    ]
                )
            chrome_args.extend(
                [
                    "--use-webgpu-adapter=swiftshader",
                    "--enable-unsafe-swiftshader",
                    "--enable-webgpu-developer-features",
                ]
            )
        chrome_args.append(page_url)
        with chrome_log.open("wb") as chr_out:
            chrome_proc = subprocess.Popen(
                chrome_args,
                stdin=subprocess.DEVNULL,
                stdout=chr_out,
                stderr=subprocess.STDOUT,
            )
            procs.append(chrome_proc)

        devtools_port = wait_for_devtools_ws(chrome_proc, chrome_log)
        env_capabilities: dict[str, Any] | None = None
        cap_deadline = time.monotonic() + 15.0
        while time.monotonic() < cap_deadline:
            try:
                targets = json.loads(
                    urllib.request.urlopen(f"http://127.0.0.1:{devtools_port}/json/list", timeout=5.0)
                    .read()
                    .decode("utf-8")
                )
                page_target = next(
                    (
                        t
                        for t in targets
                        if t.get("type") == "page"
                        and f":{http_port}/" in t.get("url", "")
                        and "webSocketDebuggerUrl" in t
                    ),
                    None,
                )
                if page_target is None:
                    time.sleep(0.2)
                    continue
                if cdp is not None:
                    cdp.close()
                cdp = SimpleWsClient(page_target["webSocketDebuggerUrl"])
                cdp.call(1, "Runtime.enable")
                adapter_expr = (
                    "(await navigator.gpu.requestAdapter({ forceFallbackAdapter: true }))"
                    " ?? (await navigator.gpu.requestAdapter())"
                    if args.software_gpu
                    else "await navigator.gpu.requestAdapter()"
                )
                eval_resp, _ = cdp.call(
                    2,
                    "Runtime.evaluate",
                    {
                        "expression": (
                            "(async () => {"
                            f" const a = navigator.gpu ? ({adapter_expr}) : null;"
                            " return {href: location.href, isolated: window.crossOriginIsolated,"
                            " sab: typeof SharedArrayBuffer === 'function', gpu: !!navigator.gpu,"
                            " adapter: !!a, adapter_device: a?.info?.device || ''};"
                            " })()"
                        ),
                        "awaitPromise": True,
                        "returnByValue": True,
                    },
                )
                val = eval_resp.get("result", {}).get("result", {}).get("value")
                if isinstance(val, dict) and f":{http_port}/" in val.get("href", ""):
                    env_capabilities = val
                    break
            except Exception:
                pass
            time.sleep(0.25)

        if not env_capabilities or not (
            env_capabilities.get("isolated")
            and env_capabilities.get("sab")
            and env_capabilities.get("gpu")
            and env_capabilities.get("adapter")
        ):
            raise WebGateError(f"Browser missing required Web capabilities: {env_capabilities}")

        def dom_click(x: float, y: float) -> None:
            cdp.call(
                None,
                "Input.dispatchMouseEvent",
                {"type": "mousePressed", "x": x, "y": y, "button": "left", "clickCount": 1},
            )
            cdp.call(
                None,
                "Input.dispatchMouseEvent",
                {"type": "mouseReleased", "x": x, "y": y, "button": "left", "clickCount": 1},
            )

        ctrl_addr = ("127.0.0.1", ctrl_port)
        join_deadline = time.monotonic() + 60.0
        last_status: dict[str, Any] = {}
        all_console_lines: list[str] = []
        consent_clicked = False
        while time.monotonic() < join_deadline:
            try:
                net_res = control_call(ctrl_addr, "net.counters", timeout=5.0)
                ren_res = control_call(ctrl_addr, "renderer.counters", timeout=5.0)
                if net_res.get("ok") and ren_res.get("ok"):
                    net = net_res["result"]
                    ren = ren_res["result"]
                    client_res = net.get("client_resources") or {}
                    selected_sources = [
                        p.get("source", "") for p in client_res.get("selected_packs", [])
                    ]
                    has_server_pack = any(s.startswith("server:") for s in selected_sources)
                    probe_val = client_res.get("selected_language_probe_value")
                    last_status = {
                        "connected": net.get("connected"),
                        "phase": net.get("phase"),
                        "uploaded_sections": ren.get("uploaded_sections", 0),
                        "visible_sections": ren.get("visible_sections", 0),
                        "frame_index": ren.get("frame_index", 0),
                        "resource_generation": client_res.get("generation"),
                        "selected_packs": selected_sources,
                        "language_probe": probe_val,
                    }
                    if not consent_clicked:
                        ui = control_call(ctrl_addr, "world.client_ui", timeout=5.0)
                        dialog = (ui.get("result") or {}).get("current_dialog") or {}
                        if dialog.get("holder_kind") == "resource_pack_consent":
                            contents = dialog["data"]["common"]["body_data"][0]["message"]["contents"]
                            if SERVER_PACK_PROMPT not in contents["text"]:
                                raise WebGateError(f"missing server pack explanation: {contents['text']!r}")
                            point, _ = cdp.call(None, "Runtime.evaluate", {
                                "expression": "(() => { const r = document.querySelector('canvas').getBoundingClientRect(); return [r.left + r.width / 2 - 70, r.bottom - 22] })()",
                                "returnByValue": True,
                            })
                            dom_click(*point["result"]["result"]["value"])
                            consent_clicked = True
                    if (
                        consent_clicked
                        and net.get("connected") is True
                        and has_server_pack
                        and probe_val == SERVER_PACK_PROBE_VALUE
                        and ren.get("uploaded_sections", 0) > 0
                        and ren.get("visible_sections", 0) > 1
                        and ren.get("frame_index", 0) >= 3
                    ):
                        break
            except Exception as exc:
                last_status = {"poll_error": str(exc)}
            if cdp is not None:
                dom_resp, console_lines = cdp.call(
                    99,
                    "Runtime.evaluate",
                    {
                        "expression": "({dom: document.getElementById('bbb-status')?.textContent || '', logs: (window.__bbb_logs || []).slice(-30)})",
                        "returnByValue": True,
                    },
                )
                all_console_lines.extend(console_lines)
                dom_val = dom_resp.get("result", {}).get("result", {}).get("value") or {}
                dom_text = dom_val.get("dom", "")
                js_logs = dom_val.get("logs", [])
                if (
                    "error" in dom_text.lower()
                    or "panicked" in dom_text.lower()
                    or "unhandled rejection" in dom_text.lower()
                    or "uncaught error" in dom_text.lower()
                ):
                    raise WebGateError(
                        f"Browser DOM reported fatal error: {dom_text}\nJS logs: {js_logs}\nCDP console: {all_console_lines[-20:]}"
                    )
            time.sleep(0.5)
        else:
            dom_info: Any = ""
            if cdp is not None:
                dom_resp, console_lines = cdp.call(
                    100,
                    "Runtime.evaluate",
                    {
                        "expression": "({dom: document.getElementById('bbb-status')?.textContent || '', logs: (window.__bbb_logs || []).slice(-40)})",
                        "returnByValue": True,
                    },
                )
                all_console_lines.extend(console_lines)
                dom_info = {"eval": dom_resp, "cdp_console": all_console_lines[-30:]}
            raise WebGateError(
                f"Timed out waiting for Web client Play join + chunk upload; last_status={last_status}; dom={dom_info}"
            )

        assert cdp is not None
        playable_resp, _ = cdp.call(
            None,
            "Runtime.evaluate",
            {
                "expression": """({
                    first_playable_ms: performance.now(),
                    hardware_concurrency: navigator.hardwareConcurrency,
                    renderer_adapter: window.__bbb_logs.find(
                        line => line.includes('renderer platform adapter selected')) ?? null,
                })""",
                "returnByValue": True,
            },
        )
        playable = playable_resp["result"]["result"]["value"]
        browser_version, _ = cdp.call(None, "Browser.getVersion")

        player_before = control_call(ctrl_addr, "world.client_local_player")["result"]
        yaw_before = player_before["pose"]["y_rot"]

        cdp.call(
            None,
            "Runtime.evaluate",
            {
                "expression": """(async () => {
                    const mod = await import('/pkg/bbb_client.js');
                    const initPayload = typeof mod.web_worker_init_payload === 'function'
                        ? mod.web_worker_init_payload()
                        : null;
                    const wasmMem = initPayload && initPayload[1] ? initPayload[1] : null;
                    const readMem = () => (wasmMem && wasmMem.buffer ? wasmMem.buffer.byteLength : 0);
                    const initialBytes = readMem();
                    const state = {
                        active: true,
                        lastTs: null,
                        frameTimesMs: [],
                        initialWasmBytes: initialBytes,
                        peakWasmBytes: initialBytes,
                        finalWasmBytes: initialBytes,
                    };
                    window.__bbb_perf_sampler = state;
                    const tick = (ts) => {
                        if (!state.active) return;
                        if (state.lastTs !== null) {
                            const dt = ts - state.lastTs;
                            if (dt >= 0) state.frameTimesMs.push(dt);
                        }
                        state.lastTs = ts;
                        const curBytes = readMem();
                        if (curBytes > state.peakWasmBytes) state.peakWasmBytes = curBytes;
                        state.finalWasmBytes = curBytes;
                        requestAnimationFrame(tick);
                    };
                    requestAnimationFrame(tick);
                    return { initialWasmBytes: initialBytes, hasWasmMemory: !!wasmMem };
                })()""",
                "awaitPromise": True,
                "returnByValue": True,
            },
        )
        cdp.call(
            None,
            "Emulation.setDeviceMetricsOverride",
            {"width": 854, "height": 480, "deviceScaleFactor": 2, "mobile": False},
        )
        time.sleep(0.25)

        def dom_key(key: str, code: str, vk: int, text: str = "") -> None:
            down: dict[str, Any] = {
                "type": "keyDown",
                "key": key,
                "code": code,
                "windowsVirtualKeyCode": vk,
                "nativeVirtualKeyCode": vk,
            }
            if text:
                down["text"] = text
            cdp.call(None, "Input.dispatchKeyEvent", down)
            cdp.call(
                None,
                "Input.dispatchKeyEvent",
                {
                    "type": "keyUp",
                    "key": key,
                    "code": code,
                    "windowsVirtualKeyCode": vk,
                    "nativeVirtualKeyCode": vk,
                },
            )

        def dom_type_text(text: str) -> None:
            punct_codes = {
                "/": ("Slash", 191),
                " ": ("Space", 32),
                ".": ("Period", 190),
                ":": ("Semicolon", 186),
                "@": ("Digit2", 50),
                "_": ("Minus", 189),
                "-": ("Minus", 189),
            }
            for ch in text:
                if "a" <= ch <= "z" or "A" <= ch <= "Z":
                    upper = ch.upper()
                    code, vk = f"Key{upper}", ord(upper)
                elif "0" <= ch <= "9":
                    code, vk = f"Digit{ch}", ord(ch)
                else:
                    code, vk = punct_codes.get(ch, ("Unidentified", ord(ch)))
                dom_key(ch, code, vk, ch)

        dom_click(427, 240)
        cdp.call(
            None,
            "Input.dispatchMouseEvent",
            {"type": "mouseWheel", "x": 427, "y": 240, "deltaX": 0, "deltaY": 120},
        )
        # Toggle F3 debug HUD, open/close Inventory (E), open/close Pause Menu (Escape),
        # send a /playsound command via Chat (T + text + Enter) to exercise lazy .ogg Web Audio,
        # and cycle camera perspective (F5 -> F5 -> F5).
        dom_key("F3", "F3", 114)
        time.sleep(0.15)
        dom_key("F3", "F3", 114)
        dom_key("e", "KeyE", 69, "e")
        time.sleep(0.2)
        dom_key("e", "KeyE", 69, "e")
        time.sleep(0.15)
        dom_key("Escape", "Escape", 27)
        time.sleep(0.2)
        dom_key("Escape", "Escape", 27)
        time.sleep(0.15)
        dom_key("t", "KeyT", 84)
        time.sleep(0.15)
        dom_type_text("/playsound minecraft:ui.button.click master @s")
        time.sleep(0.1)
        dom_key("Enter", "Enter", 13)
        time.sleep(0.35)
        for _ in range(3):
            dom_key("F5", "F5", 116)
            time.sleep(0.1)

        # Exercise live viewport resize (offscreen FrameTargets retirement) and sky-environment
        # staging updates (SkyGpu::stage_environment -> StagedCommands::write -> UploadBelt).
        cdp.call(
            None,
            "Emulation.setDeviceMetricsOverride",
            {"width": 1024, "height": 576, "deviceScaleFactor": 2, "mobile": False},
        )
        time.sleep(0.3)
        cdp.call(
            None,
            "Emulation.setDeviceMetricsOverride",
            {"width": 854, "height": 480, "deviceScaleFactor": 2, "mobile": False},
        )
        time.sleep(0.3)
        for sky_cmd in ("/time set night", "/time set day"):
            dom_key("t", "KeyT", 84)
            time.sleep(0.12)
            dom_type_text(sky_cmd)
            time.sleep(0.08)
            dom_key("Enter", "Enter", 13)
            time.sleep(0.25)

        # Multi-burst movement across chunk boundaries with idle pauses between bursts
        # so Rayon Web Workers repeatedly sleep and wake without main-thread Atomics.wait.
        assert control_call(ctrl_addr, "input.focus", {"focused": True})["ok"]
        for burst in range(3):
            assert control_call(
                ctrl_addr, "input.look_delta", {"dx": 65.0 + burst * 20.0, "dy": 5.0}
            )["ok"]
            assert control_call(ctrl_addr, "input.key", {"key": "forward", "pressed": True})["ok"]
            assert control_call(ctrl_addr, "input.key", {"key": "sprint", "pressed": True})["ok"]
            assert control_call(ctrl_addr, "input.key", {"key": "jump", "pressed": True})["ok"]
            time.sleep(0.85)
            assert control_call(ctrl_addr, "input.key", {"key": "jump", "pressed": False})["ok"]
            assert control_call(ctrl_addr, "input.key", {"key": "sprint", "pressed": False})["ok"]
            assert control_call(ctrl_addr, "input.key", {"key": "forward", "pressed": False})["ok"]
            time.sleep(0.45)

        player_after = control_call(ctrl_addr, "world.client_local_player")["result"]
        yaw_after = player_after["pose"]["y_rot"]
        pos_before = player_before["pose"]["position"]
        pos_after = player_after["pose"]["position"]
        moved_dist_sq = sum(
            (pos_after[axis] - pos_before[axis]) ** 2 for axis in ("x", "y", "z")
        )
        if abs(yaw_after - yaw_before) < 0.5 and moved_dist_sq < 0.01:
            raise WebGateError(
                f"Expected look/movement change after control input: before={player_before['pose']}, after={player_after['pose']}"
            )

        audio_counters = control_call(ctrl_addr, "audio.counters")["result"]
        if not audio_counters.get("enabled"):
            raise WebGateError(f"Expected WebAudio runtime enabled: {audio_counters}")

        # Verify reverse MCP endpoint (--mcp-bind) against the live browser tab.
        status_code, init_headers, init_body = mcp_call(
            mcp_port,
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {"name": "web-playable-gate", "version": "0.1.0"},
                },
            },
        )
        if status_code != 200 or "result" not in init_body:
            raise WebGateError(f"Reverse MCP initialize failed: HTTP {status_code} {init_body}")
        session_id = init_headers.get("mcp-session-id")
        mcp_call(
            mcp_port,
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            session_id=session_id,
        )
        tools_status, _, tools_body = mcp_call(
            mcp_port,
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
            session_id=session_id,
        )
        tool_names = {t["name"] for t in tools_body.get("result", {}).get("tools", [])}
        if tools_status != 200 or "renderer.screenshot" not in tool_names:
            raise WebGateError(f"Reverse MCP tools/list missing renderer.screenshot: {tools_body}")

        # Capture async WebGPU screenshot via reverse MCP tools/call & host path materialization.
        shot_path = work_dir / "web-playable.png"
        shot_status, _, shot_body = mcp_call(
            mcp_port,
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "renderer.screenshot",
                    "arguments": {"path": str(shot_path)},
                },
            },
            session_id=session_id,
        )
        if (
            shot_status != 200
            or shot_body.get("result", {}).get("isError")
            or not shot_path.is_file()
        ):
            raise WebGateError(f"Async WebGPU screenshot failed: HTTP {shot_status} {shot_body}")
        png_info = inspect_png(shot_path.read_bytes())

        perf_eval_resp, perf_console = cdp.call(
            None,
            "Runtime.evaluate",
            {
                "expression": """(() => {
                    const s = window.__bbb_perf_sampler;
                    if (!s) return null;
                    s.active = false;
                    return {
                        frameTimesMs: s.frameTimesMs,
                        initialWasmBytes: s.initialWasmBytes,
                        peakWasmBytes: s.peakWasmBytes,
                        finalWasmBytes: s.finalWasmBytes,
                    };
                })()""",
                "returnByValue": True,
            },
        )
        all_console_lines.extend(perf_console)
        raw_perf_sampler = perf_eval_resp.get("result", {}).get("result", {}).get("value")
        perf_telemetry = build_perf_telemetry(
            raw_perf_sampler if isinstance(raw_perf_sampler, dict) else None,
            payload_sizes,
            first_playable_ms=playable["first_playable_ms"],
            max_first_playable_ms=args.max_first_playable_ms,
            max_wasm_mib=args.max_wasm_mib,
            max_frame_p95_ms=args.max_frame_p95_ms,
        )

        final_dom_resp, final_console = cdp.call(
            None,
            "Runtime.evaluate",
            {
                "expression": """({
                    domStatus: document.getElementById('bbb-status')?.textContent || '',
                    loadingDisplay: document.getElementById('bbb-loading')?.style?.display || '',
                    fatalLatched: !!window.__bbb_fatal_latched,
                    logs: window.__bbb_logs || []
                })""",
                "returnByValue": True,
            },
        )
        all_console_lines.extend(final_console)
        final_dom = final_dom_resp.get("result", {}).get("result", {}).get("value") or {}
        js_logs: list[str] = final_dom.get("logs", [])
        bad_logs = [
            line
            for line in js_logs
            if line.startswith("[error]")
            or "panicked" in line.lower()
            or "unhandled rejection" in line.lower()
            or "uncaught error" in line.lower()
            or "invalidsoundfile" in line.lower()
            or "virtual resource not found" in line.lower()
        ]
        if (
            final_dom.get("fatalLatched")
            or final_dom.get("loadingDisplay") != "none"
            or cdp.exceptions
            or cdp.console_errors
            or bad_logs
        ):
            raise WebGateError(
                f"Post-gameplay browser health check failed:\n"
                f"  final_dom={ {k: v for k, v in final_dom.items() if k != 'logs'} }\n"
                f"  exceptions={cdp.exceptions}\n"
                f"  console_errors={cdp.console_errors}\n"
                f"  bad_logs={bad_logs}"
            )

        final_ren = control_call(ctrl_addr, "renderer.counters")["result"]
        if final_ren.get("visible_sections", 0) <= 1:
            raise WebGateError(
                f"Expected multi-chunk terrain visibility (visible_sections > 1), got: {final_ren}"
            )
        final_audio = control_call(ctrl_addr, "audio.counters")["result"]
        if final_audio.get("sound_cache_buffers", 0) < 1:
            raise WebGateError(
                f"Expected lazy .ogg WebAudio buffer decode after /playsound, got: {final_audio}"
            )

        cdp.call(None, "Page.navigate", {"url": server_url})
        rejoin_deadline = time.monotonic() + 60
        while time.monotonic() < rejoin_deadline:
            startup, _ = cdp.call(None, "Runtime.evaluate", {
                "expression": """({
                    username: localStorage.getItem('bbb.offline-player'),
                    error: window.__bbb_fatal_latched && document.getElementById('bbb-status').textContent,
                })""",
                "returnByValue": True,
            })
            state = startup.get("result", {}).get("result", {}).get("value") or {}
            if state.get("error"):
                raise WebGateError(f"Rejoin startup failed: {state['error']}")
            username = state.get("username")
            if username:
                try:
                    ui = control_call(ctrl_addr, "world.client_ui", timeout=5)
                except TimeoutError:
                    continue
                prompt = (ui.get("result") or {}).get("current_dialog") or {}
                if prompt.get("holder_kind") == "resource_pack_consent":
                    raise WebGateError("The same server asked for resource pack consent again")
                players = control_call(ctrl_addr, "world.client_player_info", timeout=5)
                entries = (players.get("result") or {}).get("entries", {})
                if offline_player_uuid(username) in entries:
                    net = control_call(ctrl_addr, "net.counters", timeout=5)
                    resources = (net.get("result") or {}).get("client_resources") or {}
                    if resources.get("selected_language_probe_value") == SERVER_PACK_PROBE_VALUE:
                        break
            time.sleep(0.5)
        else:
            raise WebGateError("The browser's default player did not rejoin with the saved pack")

        budget_error = None
        try:
            enforce_perf_budgets(perf_telemetry)
        except WebGateError as err:
            budget_error = err

        summary = {
            "ok": budget_error is None,
            "perf_budget_error": str(budget_error) if budget_error is not None else None,
            "provenance": {
                "commit": commit,
                "dirty": dirty,
                "skip_build": args.skip_build,
                "wasm_sha256": wasm_sha256,
                "browser": browser_version["result"],
                "renderer_adapter": playable["renderer_adapter"],
                "hardware_concurrency": playable["hardware_concurrency"],
                "headless": args.headless,
                "software_gpu": args.software_gpu,
                "level_seed": args.level_seed,
            },
            "resource_pack_consent": {
                "server_message": SERVER_PACK_PROMPT,
                "rejoined_with_saved_choice": True,
                "browser_default_player": username,
            },
            "browser_capabilities": env_capabilities,
            "join_status": last_status,
            "final_renderer": {
                "uploaded_sections": final_ren.get("uploaded_sections", 0),
                "visible_sections": final_ren.get("visible_sections", 0),
                "frame_index": final_ren.get("frame_index", 0),
            },
            "audio": {
                "enabled": final_audio.get("enabled"),
                "sound_cache_buffers": final_audio.get("sound_cache_buffers", 0),
            },
            "pose_before": player_before["pose"],
            "pose_after": player_after["pose"],
            "mcp_tool_count": len(tool_names),
            "screenshot": png_info,
            "perf": perf_telemetry,
        }
        if args.artifact_dir is not None:
            (work_dir / "web-playable-summary.json").write_text(
                json.dumps(summary, indent=2) + "\n", encoding="utf-8"
            )
        cdp.call(None, "Browser.close")
        chrome_proc.wait(timeout=10)
        print(json.dumps(summary, indent=2))
        if budget_error is not None:
            raise budget_error
        return 0
    finally:
        if cdp is not None:
            cdp.close()
        if pack_httpd is not None:
            pack_httpd.shutdown()
            pack_httpd.server_close()
        for proc in reversed(procs):
            terminate_process_tree(proc, timeout=5.0)
        try:
            temp_ctx.cleanup()
        except OSError:
            time.sleep(0.4)
            shutil.rmtree(runtime_dir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
