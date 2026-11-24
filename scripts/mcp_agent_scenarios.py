#!/usr/bin/env python3
"""Verify closed-loop autonomous MCP gameplay across common survival scenarios.

Supports two execution modes:
- policy: runs deterministic, zero-cheat client-MCP agents across scenarios S1-S6
  (server MCP only prepares the arena before each scenario and judges authoritative
  world/player outcomes afterwards; gameplay calls ONLY the client /mcp endpoint).
- serve: starts bbb-server + client for one scenario, writes ONLY the client /mcp
  endpoint and natural-language mission to --ready-file so an external LLM agent can
  play live, waits for --done-file, then runs the authoritative server judge.
- call: tiny CLI helper to invoke a single MCP method/tool on a running endpoint.

Supports both native desktop (`--client native`) and Web (`--client web`) clients.
"""

from __future__ import annotations

import argparse
import http.client
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
import uuid
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from bbb_tools.execution.process import free_port, terminate_process_tree  # noqa: E402
from bbb_tools.python_runner import (  # noqa: E402
    default_target_dir as shared_default_target_dir,
)
from bbb_tools.python_runner import (  # noqa: E402
    default_web_cache_dir as shared_default_web_cache_dir,
)
from bbb_tools.python_runner import (  # noqa: E402
    default_web_out_dir as shared_default_web_out_dir,
)
from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

EXE_SUFFIX = ".exe" if os.name == "nt" else ""
USERNAME = "AgentPlayer"
MCP_PROTOCOL_VERSION = "2025-06-18"
ALL_SCENARIOS = ("S1", "S2", "S3", "S4", "S5", "S6")


class ScenarioError(RuntimeError):
    pass


def default_target_dir() -> Path:
    return shared_default_target_dir() / "debug"


def default_vanilla_root() -> Path:
    return VanillaWorkspace.resolve("26.1").root


def default_wasm_dir() -> Path:
    return shared_default_web_out_dir()


@dataclass
class McpMetrics:
    calls: int = 0
    errors: int = 0
    trace: list[dict[str, Any]] = field(default_factory=list)


class McpSession:
    """Minimal HTTP JSON-RPC 2.0 MCP client with session tracking."""

    def __init__(
        self,
        host: str,
        port: int,
        *,
        client_name: str = "bbb-mcp-agent",
        timeout: float = 15.0,
        record_trace: bool = False,
    ) -> None:
        self.host = host
        self.port = port
        self.client_name = client_name
        self.timeout = timeout
        self.record_trace = record_trace
        self.session_id: str | None = None
        self.protocol_version = MCP_PROTOCOL_VERSION
        self.instructions: str | None = None
        self.req_id = 0
        self.metrics = McpMetrics()

    @property
    def url(self) -> str:
        return f"http://{self.host}:{self.port}/mcp"

    def _post(
        self,
        method: str,
        params: dict[str, Any] | None = None,
        *,
        notification: bool = False,
    ) -> tuple[Any, dict[str, str]]:
        payload: dict[str, Any] = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            payload["params"] = params
        if not notification:
            self.req_id += 1
            payload["id"] = self.req_id
        headers = {"Content-Type": "application/json", "Accept": "application/json"}
        if self.session_id is not None:
            headers["Mcp-Session-Id"] = self.session_id
            headers["Mcp-Protocol-Version"] = self.protocol_version

        conn = http.client.HTTPConnection(self.host, self.port, timeout=self.timeout)
        try:
            conn.request("POST", "/mcp", json.dumps(payload), headers)
            resp = conn.getresponse()
            body = resp.read()
            resp_headers = {k.lower(): v for k, v in resp.getheaders()}
        finally:
            conn.close()

        if not (200 <= resp.status < 300):
            raise ScenarioError(f"MCP {method} HTTP {resp.status}: {body[:600]!r}")
        if notification:
            return None, resp_headers
        decoded = json.loads(body)
        if "error" in decoded:
            raise ScenarioError(f"MCP {method} JSON-RPC error: {decoded['error']}")
        return decoded["result"], resp_headers

    def initialize(self) -> dict[str, Any]:
        result, headers = self._post(
            "initialize",
            {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": self.client_name, "version": "0.1.0"},
            },
        )
        self.session_id = headers.get("mcp-session-id")
        self.protocol_version = result.get("protocolVersion", MCP_PROTOCOL_VERSION)
        self.instructions = result.get("instructions")
        self._post("notifications/initialized", {}, notification=True)
        return result

    def list_tools(self) -> list[dict[str, Any]]:
        return self._post("tools/list", {})[0]["tools"]

    def call_raw(self, name: str, arguments: dict[str, Any] | None = None) -> dict[str, Any]:
        t0 = time.monotonic()
        res = self._post("tools/call", {"name": name, "arguments": arguments or {}})[0]
        dt_ms = round((time.monotonic() - t0) * 1000.0, 2)
        self.metrics.calls += 1
        is_err = bool(res.get("isError"))
        if is_err:
            self.metrics.errors += 1
        if self.record_trace:
            self.metrics.trace.append(
                {
                    "tool": name,
                    "arguments": arguments or {},
                    "is_error": is_err,
                    "elapsed_ms": dt_ms,
                }
            )
        return res

    def call(self, name: str, arguments: dict[str, Any] | None = None) -> Any:
        res = self.call_raw(name, arguments)
        if res.get("isError"):
            text = " ".join(c.get("text", "") for c in res.get("content", []))
            raise ScenarioError(f"MCP tool {name} failed: {text}")
        if "structuredContent" in res:
            return res["structuredContent"]
        for item in res.get("content", []):
            if item.get("type") == "text":
                try:
                    return json.loads(item["text"])
                except ValueError:
                    return item["text"]
        return res


class ServerAdmin:
    """Server-side setup and independent verdict helper via bbb-server MCP."""

    def __init__(self, port: int) -> None:
        self.mcp = McpSession("127.0.0.1", port, client_name="scenario-judge", timeout=30.0)
        self.mcp.initialize()

    def cmd(self, command: str, *, allow_fail: bool = False) -> dict[str, Any]:
        res = self.mcp.call("command.execute", {"command": command})["command"]
        if res.get("result") is None and not allow_fail:
            raise ScenarioError(f"server command failed: {command} -> {res}")
        return res

    def cmds(self, *commands: str) -> None:
        for command in commands:
            self.cmd(command)

    def blocks(self, positions: list[dict[str, int]]) -> list[dict[str, Any]]:
        return self.mcp.call(
            "world.blocks", {"dimension": "minecraft:overworld", "positions": positions}
        )["blocks"]

    def save_and_stop(self) -> None:
        try:
            self.mcp.call("server.save", {"flush": True})
        except Exception:
            pass
        try:
            self.mcp.call("server.stop", {})
        except Exception:
            pass


# --- Pure Client-MCP Gameplay Primitives (Zero Server Cheats) ---------------


def ensure_gameplay_ready(mcp: McpSession) -> None:
    status = mcp.call("app.status")
    app = status["app"]
    if app.get("death_screen") is not None:
        mcp.call("net.perform_respawn")
        mcp.call("sim.wait_ticks", {"ticks": 6})
    for _ in range(3):
        mcp.call("input.focus", {"focused": True, "capture_cursor": True})
        status = mcp.call("app.status")
        if status["app"]["input_focused"] and status["app"]["cursor_captured"]:
            return
        mcp.call("input.key", {"key": "escape", "pressed": True})
        mcp.call("input.key", {"key": "escape", "pressed": False})
        mcp.call("sim.wait_ticks", {"ticks": 2})
    status = mcp.call("app.status")
    if not (status["app"]["input_focused"] and status["app"]["cursor_captured"]):
        raise ScenarioError(f"failed to enter gameplay focus/capture: {status['app']}")


def player_pos(mcp: McpSession) -> tuple[float, float, float]:
    pose = mcp.call("world.client_local_player")["pose"]
    p = pose["position"]
    return float(p["x"]), float(p["y"]), float(p["z"])


def inventory_items(mcp: McpSession) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    inv = mcp.call("world.client_inventory")
    slots = inv["inventory_menu"]["slots"]
    return inv, slots


def count_item(slots: list[dict[str, Any]], item_name: str) -> int:
    total = 0
    for row in slots:
        item = row["item"]
        if item.get("item_name") == item_name and item.get("count", 0) > 0:
            total += int(item["count"])
    return total


def find_slot_with_item(
    slots: list[dict[str, Any]],
    item_name: str,
    *,
    min_slot: int = 9,
    max_slot: int = 44,
) -> int | None:
    for row in slots:
        slot = int(row["slot"])
        if min_slot <= slot <= max_slot:
            item = row["item"]
            if item.get("item_name") == item_name and item.get("count", 0) > 0:
                return slot
    return None


def find_empty_slot(
    slots: list[dict[str, Any]], *, min_slot: int = 36, max_slot: int = 44
) -> int | None:
    for row in slots:
        slot = int(row["slot"])
        if min_slot <= slot <= max_slot and row["item"].get("count", 0) == 0:
            return slot
    return None


def equip_item_on_hotbar(mcp: McpSession, item_name: str) -> int:
    _, slots = inventory_items(mcp)
    slot = find_slot_with_item(slots, item_name, min_slot=36, max_slot=44)
    if slot is None:
        src = find_slot_with_item(slots, item_name, min_slot=9, max_slot=35)
        if src is None:
            raise ScenarioError(f"item {item_name} not in inventory")
        dst = find_empty_slot(slots, min_slot=36, max_slot=44) or 36
        hotbar_idx = dst - 36
        mcp.call("input.key", {"key": "inventory", "pressed": True})
        mcp.call("input.key", {"key": "inventory", "pressed": False})
        mcp.call("sim.wait_ticks", {"ticks": 2})
        mcp.call(
            "net.container_click_slot",
            {"slot_num": src, "button_num": hotbar_idx, "input": "swap"},
        )
        mcp.call("sim.wait_ticks", {"ticks": 2})
        mcp.call("input.key", {"key": "escape", "pressed": True})
        mcp.call("input.key", {"key": "escape", "pressed": False})
        mcp.call("sim.wait_ticks", {"ticks": 2})
        ensure_gameplay_ready(mcp)
        slot = dst
    hotbar_index = slot - 36
    mcp.call("net.set_held_slot", {"slot": hotbar_index})
    mcp.call("sim.wait_ticks", {"ticks": 2})
    return hotbar_index


def scan_local_blocks(
    mcp: McpSession,
    *,
    radius_xz: int = 6,
    dy_min: int = -1,
    dy_max: int = 3,
    target_names: set[str],
) -> list[dict[str, Any]]:
    px, py, pz = player_pos(mcp)
    cx, cy, cz = math.floor(px), math.floor(py), math.floor(pz)
    coords: list[dict[str, int]] = []
    for dy in range(dy_min, dy_max + 1):
        for dz in range(-radius_xz, radius_xz + 1):
            for dx in range(-radius_xz, radius_xz + 1):
                coords.append({"x": cx + dx, "y": cy + dy, "z": cz + dz})
    found: list[dict[str, Any]] = []
    for i in range(0, len(coords), 256):
        batch = mcp.call(
            "world.probe_blocks", {"positions": coords[i : i + 256], "skip_air": True}
        )["blocks"]
        for row in batch:
            if row.get("loaded") and row.get("block_name") in target_names:
                found.append(row)
    found.sort(
        key=lambda b: (b["x"] + 0.5 - px) ** 2
        + (b["y"] + 0.5 - (py + 1.0)) ** 2
        + (b["z"] + 0.5 - pz) ** 2
    )
    return found


def walk_to_xz(
    mcp: McpSession,
    tx: float,
    tz: float,
    *,
    stop_dist: float = 1.6,
    max_steps: int = 18,
) -> None:
    ensure_gameplay_ready(mcp)
    for _ in range(max_steps):
        px, py, pz = player_pos(mcp)
        dist = math.hypot(tx - px, tz - pz)
        if dist <= stop_dist:
            return
        mcp.call("input.look_at", {"x": tx, "y": py + 1.62, "z": tz})
        ticks = max(2, min(14, int(round((dist - stop_dist * 0.6) / 0.215))))
        mcp.call("input.key", {"key": "forward", "pressed": True, "hold_ticks": ticks})
        pose = mcp.call("world.client_local_player")["pose"]
        if pose.get("horizontal_collision"):
            mcp.call("input.key", {"key": "jump", "pressed": True})
            mcp.call("input.key", {"key": "forward", "pressed": True, "hold_ticks": 6})
            mcp.call("input.key", {"key": "jump", "pressed": False})


def mine_block(
    mcp: McpSession,
    bx: int,
    by: int,
    bz: int,
    *,
    expected_name: str,
    hold_ticks_per_try: int = 45,
    max_tries: int = 3,
) -> None:
    walk_to_xz(mcp, bx + 0.5, bz + 0.5, stop_dist=2.6)
    ensure_gameplay_ready(mcp)
    for _ in range(max_tries):
        cur = mcp.call("world.probe_block", {"x": bx, "y": by, "z": bz})
        if cur is None or cur.get("block_name") == "minecraft:air":
            return
        mcp.call("input.look_at", {"x": bx + 0.5, "y": by + 0.5, "z": bz + 0.5})
        target = mcp.call("world.crosshair_target")
        if target.get("kind") != "block" or target["block"]["pos"] != {
            "x": bx,
            "y": by,
            "z": bz,
        }:
            # Try aiming slightly closer to the visible side face.
            px, py, pz = player_pos(mcp)
            dx = 0.2 if px > bx + 0.5 else -0.2
            dz = 0.2 if pz > bz + 0.5 else -0.2
            mcp.call("input.look_at", {"x": bx + 0.5 + dx, "y": by + 0.5, "z": bz + 0.5 + dz})
            target = mcp.call("world.crosshair_target")
        if target.get("kind") != "block" or target["block"]["block_name"] != expected_name:
            walk_to_xz(mcp, bx + 0.5, bz + 0.5, stop_dist=2.0, max_steps=6)
            mcp.call("input.look_at", {"x": bx + 0.5, "y": by + 0.5, "z": bz + 0.5})
        mcp.call(
            "input.mouse_button",
            {"button": "left", "pressed": True, "hold_ticks": hold_ticks_per_try},
        )
        mcp.call("sim.wait_ticks", {"ticks": 4})
        cur = mcp.call("world.probe_block", {"x": bx, "y": by, "z": bz})
        if cur is None or cur.get("block_name") == "minecraft:air":
            return
    raise ScenarioError(f"failed to break {expected_name} at ({bx}, {by}, {bz})")


def collect_dropped_items(
    mcp: McpSession, item_name: str, *, min_count: int = 1, max_attempts: int = 8
) -> int:
    for _ in range(max_attempts):
        _, slots = inventory_items(mcp)
        have = count_item(slots, item_name)
        if have >= min_count:
            return have
        nearby = mcp.call("world.nearby_entities", {"max_distance": 12.0, "limit": 32})["entities"]
        drops = [
            e
            for e in nearby
            if e.get("entity_type") == "minecraft:item"
            and (e.get("item") or {}).get("item_name") == item_name
        ]
        if drops:
            pos = drops[0]["position"]
            walk_to_xz(mcp, float(pos["x"]), float(pos["z"]), stop_dist=0.5, max_steps=8)
        mcp.call("sim.wait_ticks", {"ticks": 6})
    _, slots = inventory_items(mcp)
    have = count_item(slots, item_name)
    if have < min_count:
        raise ScenarioError(f"expected >= {min_count} {item_name}, got {have}")
    return have


def container_slots_by_name(inv: dict[str, Any]) -> tuple[list[dict[str, Any]], int]:
    """Return active menu slots and first player-inventory slot index."""
    oc = inv.get("open_container")
    if oc is not None:
        return oc["slots"], oc["menu_type"]
    return inv["inventory_menu"]["slots"], "player_inventory"


def click_slot(mcp: McpSession, slot_num: int, button_num: int = 0, mode: str = "pickup") -> None:
    mcp.call(
        "net.container_click_slot",
        {"slot_num": slot_num, "button_num": button_num, "input": mode},
    )
    mcp.call("sim.wait_ticks", {"ticks": 2})


def place_one_from_slot_into(
    mcp: McpSession, item_name: str, dest_slots: list[int], *, player_slot_start: int
) -> None:
    """Pick up a stack of item_name from player slots, place 1 into each dest slot, put back."""
    inv = mcp.call("world.client_inventory")
    slots = (
        inv["open_container"]["slots"]
        if inv.get("open_container") is not None
        else inv["inventory_menu"]["slots"]
    )
    src = find_slot_with_item(slots, item_name, min_slot=player_slot_start, max_slot=999)
    if src is None:
        raise ScenarioError(f"missing {item_name} in slots >= {player_slot_start}")
    click_slot(mcp, src, 0, "pickup")
    for dst in dest_slots:
        click_slot(mcp, dst, 1, "pickup")
    # Put remainder back if cursor still holds items.
    inv = mcp.call("world.client_inventory")
    if inv["cursor_item"].get("count", 0) > 0:
        click_slot(mcp, src, 0, "pickup")


def open_player_inventory(mcp: McpSession) -> None:
    mcp.call("input.key", {"key": "inventory", "pressed": True})
    mcp.call("input.key", {"key": "inventory", "pressed": False})
    mcp.call("sim.wait_ticks", {"ticks": 3})


def close_screen(mcp: McpSession) -> None:
    mcp.call("input.key", {"key": "escape", "pressed": True})
    mcp.call("input.key", {"key": "escape", "pressed": False})
    mcp.call("sim.wait_ticks", {"ticks": 3})
    ensure_gameplay_ready(mcp)


def craft_in_player_2x2(mcp: McpSession, recipe: dict[str, list[int]], expected_output: str) -> None:
    open_player_inventory(mcp)
    for item_name, grid_slots in recipe.items():
        place_one_from_slot_into(mcp, item_name, grid_slots, player_slot_start=9)
    inv = mcp.call("world.client_inventory")
    out_item = inv["inventory_menu"]["slots"][0]["item"]
    if out_item.get("item_name") != expected_output or out_item.get("count", 0) <= 0:
        raise ScenarioError(f"expected 2x2 output {expected_output}, got {out_item}")
    click_slot(mcp, 0, 0, "quick_move")
    close_screen(mcp)


def craft_in_table_3x3(mcp: McpSession, recipe: dict[str, list[int]], expected_output: str) -> None:
    inv = mcp.call("world.client_inventory")
    oc = inv.get("open_container")
    if oc is None or oc.get("menu_type") != "minecraft:crafting":
        raise ScenarioError(f"crafting table not open: {oc}")
    for item_name, grid_slots in recipe.items():
        place_one_from_slot_into(mcp, item_name, grid_slots, player_slot_start=10)
    inv = mcp.call("world.client_inventory")
    out_item = inv["open_container"]["slots"][0]["item"]
    if out_item.get("item_name") != expected_output or out_item.get("count", 0) <= 0:
        raise ScenarioError(f"expected 3x3 output {expected_output}, got {out_item}")
    click_slot(mcp, 0, 0, "quick_move")
    close_screen(mcp)


def take_shot(mcp: McpSession, out_dir: Path, name: str) -> str:
    shot_path = out_dir / "shots" / f"{name}.png"
    shot_path.parent.mkdir(parents=True, exist_ok=True)
    res = mcp.call("renderer.screenshot", {"path": str(shot_path)})
    if not res.get("completed") or not shot_path.is_file():
        raise ScenarioError(f"screenshot {name} did not complete: {res}")
    return str(shot_path)


# --- Scenario Definitions (Setup, Pure-Client Policy, Independent Verdict) ---


SCENARIO_MISSIONS: dict[str, str] = {
    "S1": (
        "Survival opening: find the oak tree nearby, chop oak logs, pick them up, "
        "craft oak planks, sticks, and a crafting table, place the crafting table on the ground, "
        "open it to craft a wooden pickaxe, and hold the wooden pickaxe in your main hand."
    ),
    "S2": (
        "Mining, smelting & torch: mine the nearby stone block with your stone pickaxe and "
        "collect the cobblestone, smelt your raw iron in the nearby furnace using coal until "
        "you obtain an iron ingot in your inventory, craft torches from coal and stick, and "
        "place a wall torch on the side face of the nearby stone wall."
    ),
    "S3": (
        "Combat & sustenance: defeat the zombie inside the enclosed stone arena using your "
        "iron sword, then eat cooked beef until your food level rises."
    ),
    "S4": (
        "Chest & villager trade: open the nearby chest to retrieve emeralds, then trade "
        "with the nearby villager to obtain bread in your inventory."
    ),
    "S5": (
        "Surface building & sign: scan the flat ground around you, place at least 3 oak "
        "planks blocks as a shelter wall/pillar, then place an oak sign and write text on it."
    ),
    "S6": (
        "Recovery & respawn: recover from an opened pause screen back into first-person "
        "control, survive/recover after dying by respawning, then navigate back to pick up "
        "your dropped diamond sword."
    ),
}


def reset_arena(admin: ServerAdmin, *, x0: int = 0, z0: int = 0) -> None:
    admin.cmds(
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "gamerule advance_weather false",
        "gamerule keep_inventory false",
        "time set 6000",
        "weather clear",
    )
    # Commands that return failure when already in the target state / nothing matched.
    for cmd in (
        "difficulty normal",
        f"kill @e[type=!minecraft:player,x={x0},y=64,z={z0},distance=..40]",
        f"fill {x0 - 12} 62 {z0 - 12} {x0 + 12} 62 {z0 + 12} minecraft:bedrock",
        f"fill {x0 - 12} 63 {z0 - 12} {x0 + 12} 63 {z0 + 12} minecraft:stone",
        f"fill {x0 - 12} 64 {z0 - 12} {x0 + 12} 72 {z0 + 12} minecraft:air",
        f"gamemode survival {USERNAME}",
        f"clear {USERNAME}",
        f"effect clear {USERNAME}",
        f"effect give {USERNAME} minecraft:instant_health 1 10 true",
        f"effect give {USERNAME} minecraft:saturation 1 10 true",
    ):
        admin.cmd(cmd, allow_fail=True)
    admin.cmds(
        f"spawnpoint {USERNAME} {x0} 64 {z0}",
        f"tp {USERNAME} {x0 + 0.5} 64 {z0 + 0.5} 0 0",
    )


def setup_s1(admin: ServerAdmin) -> None:
    reset_arena(admin)
    # Build a small oak tree 3 blocks ahead (+Z).
    admin.cmds(
        "setblock 0 64 4 minecraft:oak_log",
        "setblock 0 65 4 minecraft:oak_log",
        "setblock 0 66 4 minecraft:oak_log",
        "setblock 0 67 4 minecraft:oak_leaves[persistent=true]",
        "fill -1 66 3 1 66 5 minecraft:oak_leaves[persistent=true] replace minecraft:air",
    )


def policy_s1(mcp: McpSession, out_dir: Path) -> dict[str, Any]:
    ensure_gameplay_ready(mcp)
    # 1. Discover oak logs dynamically via client world.probe_blocks.
    logs = scan_local_blocks(mcp, radius_xz=6, dy_min=0, dy_max=3, target_names={"minecraft:oak_log"})
    if len(logs) < 3:
        raise ScenarioError(f"S1: expected >=3 oak logs nearby, found {len(logs)}")
    # Chop from bottom up.
    logs.sort(key=lambda b: b["y"])
    for log in logs[:3]:
        mine_block(
            mcp,
            log["x"],
            log["y"],
            log["z"],
            expected_name="minecraft:oak_log",
            hold_ticks_per_try=68,
        )
    got_logs = collect_dropped_items(mcp, "minecraft:oak_log", min_count=3)

    # 2. Craft planks (x3 -> 12 planks), sticks (4 sticks), and crafting_table in 2x2 inventory.
    craft_in_player_2x2(mcp, {"minecraft:oak_log": [1]}, "minecraft:oak_planks")
    craft_in_player_2x2(mcp, {"minecraft:oak_log": [1]}, "minecraft:oak_planks")
    craft_in_player_2x2(mcp, {"minecraft:oak_log": [1]}, "minecraft:oak_planks")
    craft_in_player_2x2(mcp, {"minecraft:oak_planks": [1, 3]}, "minecraft:stick")
    craft_in_player_2x2(mcp, {"minecraft:oak_planks": [1, 2, 3, 4]}, "minecraft:crafting_table")

    # 3. Scan flat floor via world.probe_surface and place crafting table on an adjacent empty tile.
    px, py, pz = player_pos(mcp)
    cx, cz = math.floor(px), math.floor(pz)
    surface = mcp.call(
        "world.probe_surface",
        {"min_x": cx - 2, "min_z": cz - 2, "max_x": cx + 2, "max_z": cz + 2, "max_y": int(py)},
    )["columns"]
    place_col = next(
        col
        for col in surface
        if col["loaded"]
        and col["top_y"] == int(py) - 1
        and not col["is_fluid"]
        and abs(col["x"] - cx) + abs(col["z"] - cz) == 2
    )
    equip_item_on_hotbar(mcp, "minecraft:crafting_table")
    mcp.call(
        "input.look_at",
        {"x": place_col["x"] + 0.5, "y": place_col["top_y"] + 1.0, "z": place_col["z"] + 0.5},
    )
    target = mcp.call("world.crosshair_target")
    if target.get("kind") != "block" or target["block"]["face"] != "up":
        raise ScenarioError(f"S1: expected up face for crafting table placement, got {target}")
    table_pos = target["block"]["adjacent_pos"]
    mcp.call("input.mouse_tap", {"button": "right"})
    mcp.call("sim.wait_ticks", {"ticks": 4})

    # 4. Open placed crafting table and craft wooden_pickaxe (3 planks across top + 2 sticks).
    mcp.call(
        "input.look_at",
        {"x": table_pos["x"] + 0.5, "y": table_pos["y"] + 0.5, "z": table_pos["z"] + 0.5},
    )
    mcp.call("input.mouse_tap", {"button": "right"})
    mcp.call("sim.wait_ticks", {"ticks": 4})
    craft_in_table_3x3(
        mcp,
        {"minecraft:oak_planks": [1, 2, 3], "minecraft:stick": [5, 8]},
        "minecraft:wooden_pickaxe",
    )
    equip_item_on_hotbar(mcp, "minecraft:wooden_pickaxe")
    shot = take_shot(mcp, out_dir, "s1_wood_to_pickaxe")
    return {"logs_collected": got_logs, "crafting_table_pos": table_pos, "screenshot": shot}


def judge_s1(admin: ServerAdmin) -> dict[str, Any]:
    held = admin.cmd(
        f"execute if items entity {USERNAME} weapon.mainhand minecraft:wooden_pickaxe"
    )["result"]
    if held != 1:
        raise ScenarioError(f"S1 judge failed: wooden_pickaxe not in mainhand (result={held})")
    # Verify a crafting_table exists on the floor somewhere in the arena.
    found_table = False
    positions = [{"x": x, "y": 64, "z": z} for z in range(-6, 7) for x in range(-6, 7)]
    for row in admin.blocks(positions):
        state = row.get("state") or {}
        if state.get("name") == "minecraft:crafting_table":
            found_table = True
            break
    if not found_table:
        raise ScenarioError("S1 judge failed: no placed crafting_table found in arena")
    return {"wooden_pickaxe_held": True, "crafting_table_placed": True}


def setup_s2(admin: ServerAdmin) -> None:
    reset_arena(admin)
    admin.cmds(
        "setblock 0 64 3 minecraft:stone",
        "setblock -2 64 2 minecraft:furnace[facing=east]",
        "fill 2 64 2 2 66 3 minecraft:stone_bricks",
        f"item replace entity {USERNAME} hotbar.0 with minecraft:stone_pickaxe 1",
        f"item replace entity {USERNAME} hotbar.1 with minecraft:raw_iron 1",
        f"item replace entity {USERNAME} hotbar.2 with minecraft:coal 2",
        f"item replace entity {USERNAME} hotbar.3 with minecraft:stick 1",
    )


def policy_s2(mcp: McpSession, out_dir: Path) -> dict[str, Any]:
    ensure_gameplay_ready(mcp)
    # 1. Mine stone block with stone pickaxe and collect cobblestone.
    equip_item_on_hotbar(mcp, "minecraft:stone_pickaxe")
    stones = scan_local_blocks(mcp, radius_xz=5, dy_min=0, dy_max=1, target_names={"minecraft:stone"})
    if not stones:
        raise ScenarioError("S2: no stone block found nearby")
    st = stones[0]
    mine_block(mcp, st["x"], st["y"], st["z"], expected_name="minecraft:stone", hold_ticks_per_try=15)
    cobble = collect_dropped_items(mcp, "minecraft:cobblestone", min_count=1)

    # 2. Find furnace, open it, load raw_iron (slot 0) and coal (slot 1), wait for smelting.
    furnaces = scan_local_blocks(
        mcp, radius_xz=5, dy_min=0, dy_max=1, target_names={"minecraft:furnace"}
    )
    if not furnaces:
        raise ScenarioError("S2: no furnace found nearby")
    fn = furnaces[0]
    walk_to_xz(mcp, fn["x"] + 0.5, fn["z"] + 0.5, stop_dist=2.4)
    mcp.call("input.look_at", {"x": fn["x"] + 0.5, "y": fn["y"] + 0.5, "z": fn["z"] + 0.5})
    mcp.call("input.mouse_tap", {"button": "right"})
    mcp.call("sim.wait_ticks", {"ticks": 4})
    inv = mcp.call("world.client_inventory")
    if (inv.get("open_container") or {}).get("menu_type") != "minecraft:furnace":
        raise ScenarioError(f"S2: furnace did not open: {inv.get('open_container')}")
    place_one_from_slot_into(mcp, "minecraft:raw_iron", [0], player_slot_start=3)
    place_one_from_slot_into(mcp, "minecraft:coal", [1], player_slot_start=3)
    # Smelting takes 200 ticks (10 seconds); wait in bounded chunks and check output slot 2.
    smelted = False
    for _ in range(5):
        mcp.call("sim.wait_ticks", {"ticks": 55})
        inv = mcp.call("world.client_inventory")
        out = inv["open_container"]["slots"][2]["item"]
        if out.get("item_name") == "minecraft:iron_ingot" and out.get("count", 0) >= 1:
            click_slot(mcp, 2, 0, "quick_move")
            smelted = True
            break
    close_screen(mcp)
    if not smelted:
        raise ScenarioError("S2: furnace did not finish smelting iron_ingot")

    # 3. Craft torch in 2x2 grid (coal over stick) and place on horizontal wall face.
    craft_in_player_2x2(mcp, {"minecraft:coal": [1], "minecraft:stick": [3]}, "minecraft:torch")
    equip_item_on_hotbar(mcp, "minecraft:torch")
    wall_blocks = scan_local_blocks(
        mcp, radius_xz=6, dy_min=1, dy_max=1, target_names={"minecraft:stone_bricks"}
    )
    if not wall_blocks:
        raise ScenarioError("S2: no stone_bricks wall found")
    wb = wall_blocks[0]
    walk_to_xz(mcp, wb["x"] - 1.5, wb["z"] + 0.5, stop_dist=1.0)
    # Aim at the west face of the wall block.
    mcp.call("input.look_at", {"x": wb["x"] + 0.01, "y": wb["y"] + 0.5, "z": wb["z"] + 0.5})
    target = mcp.call("world.crosshair_target")
    if target.get("kind") != "block" or target["block"]["face"] not in {
        "north",
        "south",
        "east",
        "west",
    }:
        raise ScenarioError(f"S2: expected horizontal wall face, got {target}")
    torch_pos = target["block"]["adjacent_pos"]
    mcp.call("input.mouse_tap", {"button": "right"})
    mcp.call("sim.wait_ticks", {"ticks": 4})
    shot = take_shot(mcp, out_dir, "s2_mining_smelting_torch")
    return {"cobblestone": cobble, "torch_pos": torch_pos, "screenshot": shot}


def judge_s2(admin: ServerAdmin) -> dict[str, Any]:
    cobble = admin.cmd(f"execute if items entity {USERNAME} container.* minecraft:cobblestone")[
        "result"
    ]
    iron = admin.cmd(f"execute if items entity {USERNAME} container.* minecraft:iron_ingot")[
        "result"
    ]
    if not cobble or not iron:
        raise ScenarioError(f"S2 judge failed: cobble={cobble}, iron={iron}")
    positions = [{"x": x, "y": 65, "z": z} for z in range(0, 6) for x in range(-1, 5)]
    wall_torch = any(
        (b.get("state") or {}).get("name") == "minecraft:wall_torch" for b in admin.blocks(positions)
    )
    if not wall_torch:
        raise ScenarioError("S2 judge failed: no minecraft:wall_torch placed on wall")
    return {"cobblestone": cobble, "iron_ingot": iron, "wall_torch_placed": True}


def setup_s3(admin: ServerAdmin) -> None:
    reset_arena(admin)
    # Lit, roofed cobblestone pen so sunlight never burns the zombie.
    admin.cmds(
        "fill -4 64 -4 4 67 4 minecraft:cobblestone hollow",
        "fill -3 64 -3 3 66 3 minecraft:air",
        "setblock 0 67 0 minecraft:sea_lantern",
        f"tp {USERNAME} 0.5 64 -2.0 0 0",
        f"item replace entity {USERNAME} hotbar.0 with minecraft:iron_sword 1",
        f"item replace entity {USERNAME} hotbar.1 with minecraft:cooked_beef 4",
        # Apply brief strong hunger so saturation drains and food drops below 20.
        f"effect give {USERNAME} minecraft:hunger 4 250 true",
        "summon minecraft:zombie 0.5 64 2.0 {PersistenceRequired:1b}",
    )


def policy_s3(mcp: McpSession, out_dir: Path) -> dict[str, Any]:
    ensure_gameplay_ready(mcp)
    equip_item_on_hotbar(mcp, "minecraft:iron_sword")
    hits = 0
    for _ in range(35):
        nearby = mcp.call("world.nearby_entities", {"max_distance": 12.0, "limit": 16})["entities"]
        zombies = [e for e in nearby if e.get("entity_type") == "minecraft:zombie"]
        if not zombies:
            break
        z = zombies[0]
        pos = z["position"]
        if float(z["distance"]) > 2.6:
            walk_to_xz(mcp, float(pos["x"]), float(pos["z"]), stop_dist=2.2, max_steps=3)
        mcp.call("input.look_at", {"entity_id": z["id"], "anchor": "center"})
        target = mcp.call("world.crosshair_target")
        if target.get("kind") == "entity" and target["entity"]["entity_id"] == z["id"]:
            mcp.call("input.mouse_tap", {"button": "left"})
            hits += 1
            mcp.call("sim.wait_ticks", {"ticks": 12})
        else:
            mcp.call("sim.wait_ticks", {"ticks": 4})

    # Wait for brief setup hunger effect to expire, then eat cooked_beef.
    self_ents = mcp.call(
        "world.nearby_entities",
        {"max_distance": 1.0, "limit": 8, "include_local_player": True},
    )["entities"]
    local_id = next(
        (e["id"] for e in self_ents if e.get("entity_type") == "minecraft:player" and e["distance"] < 0.01),
        None,
    )
    for _ in range(10):
        status = mcp.call("world.probe_entity_status", {"id": local_id}) if local_id is not None else None
        if not (status and status.get("mob_effects")):
            break
        mcp.call("sim.wait_ticks", {"ticks": 10})

    food_before = int(mcp.call("world.client_local_player")["health"]["food"])
    equip_item_on_hotbar(mcp, "minecraft:cooked_beef")
    mcp.call("input.mouse_button", {"button": "right", "pressed": True, "hold_ticks": 38})
    mcp.call("sim.wait_ticks", {"ticks": 4})
    food_after = int(mcp.call("world.client_local_player")["health"]["food"])
    if food_after <= food_before and food_before < 20:
        # Eat a second time if first hold was interrupted.
        mcp.call("input.mouse_button", {"button": "right", "pressed": True, "hold_ticks": 38})
        mcp.call("sim.wait_ticks", {"ticks": 4})
        food_after = int(mcp.call("world.client_local_player")["health"]["food"])
    shot = take_shot(mcp, out_dir, "s3_combat_eat")
    return {
        "sword_hits": hits,
        "food_before": food_before,
        "food_after": food_after,
        "screenshot": shot,
    }


def judge_s3(admin: ServerAdmin) -> dict[str, Any]:
    remaining = admin.cmd("execute if entity @e[type=minecraft:zombie,distance=..20]", allow_fail=True)
    if remaining.get("result"):
        raise ScenarioError(f"S3 judge failed: zombie still alive ({remaining})")
    beef_left = admin.cmd(f"execute if items entity {USERNAME} container.* minecraft:cooked_beef")[
        "result"
    ]
    if beef_left is None or beef_left >= 4:
        raise ScenarioError(f"S3 judge failed: expected cooked_beef consumed, left={beef_left}")
    return {"zombie_defeated": True, "beef_remaining": beef_left}


def setup_s4(admin: ServerAdmin) -> None:
    reset_arena(admin)
    admin.cmds(
        "setblock -2 64 2 minecraft:chest[facing=south]",
        "item replace block -2 64 2 container.0 with minecraft:emerald 16",
        (
            'summon villager 2.5 64 2.5 '
            '{NoAI:1b,VillagerData:{profession:"minecraft:farmer",level:2},'
            'Offers:{Recipes:['
            '{buy:{id:"minecraft:emerald",count:1},sell:{id:"minecraft:bread",count:3},maxUses:12}'
            ']}}'
        ),
    )


def policy_s4(mcp: McpSession, out_dir: Path) -> dict[str, Any]:
    ensure_gameplay_ready(mcp)
    # 1. Open chest and quick_move emeralds into inventory.
    chests = scan_local_blocks(mcp, radius_xz=6, dy_min=0, dy_max=1, target_names={"minecraft:chest"})
    if not chests:
        raise ScenarioError("S4: chest not found")
    ch = chests[0]
    walk_to_xz(mcp, ch["x"] + 0.5, ch["z"] + 0.5, stop_dist=2.4)
    mcp.call("input.look_at", {"x": ch["x"] + 0.5, "y": ch["y"] + 0.5, "z": ch["z"] + 0.5})
    mcp.call("input.mouse_tap", {"button": "right"})
    mcp.call("sim.wait_ticks", {"ticks": 4})
    inv = mcp.call("world.client_inventory")
    oc = inv.get("open_container")
    if oc is None or oc.get("menu_type") != "minecraft:generic_9x3":
        raise ScenarioError(f"S4: chest did not open: {oc}")
    em_slot = find_slot_with_item(oc["slots"], "minecraft:emerald", min_slot=0, max_slot=26)
    if em_slot is None:
        raise ScenarioError("S4: no emeralds in chest")
    click_slot(mcp, em_slot, 0, "quick_move")
    close_screen(mcp)

    # 2. Approach villager, open merchant menu, exercise input.scroll, select bread offer, take bread.
    nearby = mcp.call("world.nearby_entities", {"max_distance": 12.0, "limit": 16})["entities"]
    villagers = [e for e in nearby if e.get("entity_type") == "minecraft:villager"]
    if not villagers:
        raise ScenarioError("S4: villager not found")
    v = villagers[0]
    walk_to_xz(mcp, float(v["position"]["x"]), float(v["position"]["z"]), stop_dist=2.4)
    mcp.call("input.look_at", {"entity_id": v["id"], "anchor": "center"})
    mcp.call("input.mouse_tap", {"button": "right"})
    mcp.call("sim.wait_ticks", {"ticks": 5})
    inv = mcp.call("world.client_inventory")
    oc = inv.get("open_container")
    if oc is None or oc.get("menu_type") != "minecraft:merchant":
        raise ScenarioError(f"S4: villager trade screen did not open: {oc}")
    mcp.call("input.scroll", {"dy": -1.0})
    offers = (oc.get("merchant_offers") or {}).get("offers", [])
    bread_idx = next(
        (
            idx
            for idx, offer in enumerate(offers)
            if (offer.get("sell") or {}).get("item_name") == "minecraft:bread"
        ),
        None,
    )
    if bread_idx is None:
        raise ScenarioError(f"S4: bread offer not found in {offers}")
    mcp.call("net.select_trade", {"item": bread_idx})
    mcp.call("sim.wait_ticks", {"ticks": 3})
    click_slot(mcp, 2, 0, "quick_move")
    close_screen(mcp)
    shot = take_shot(mcp, out_dir, "s4_chest_trade")
    return {"trade_index": bread_idx, "screenshot": shot}


def judge_s4(admin: ServerAdmin) -> dict[str, Any]:
    bread = admin.cmd(f"execute if items entity {USERNAME} container.* minecraft:bread")["result"]
    if not bread or bread < 3:
        raise ScenarioError(f"S4 judge failed: expected >=3 bread, got {bread}")
    return {"bread_count": bread}


def setup_s5(admin: ServerAdmin) -> None:
    reset_arena(admin)
    admin.cmds(
        f"item replace entity {USERNAME} hotbar.0 with minecraft:oak_planks 32",
        f"item replace entity {USERNAME} hotbar.1 with minecraft:oak_sign 2",
    )


def policy_s5(mcp: McpSession, out_dir: Path) -> dict[str, Any]:
    ensure_gameplay_ready(mcp)
    px, py, pz = player_pos(mcp)
    cx, cz = math.floor(px), math.floor(pz)
    surface = mcp.call(
        "world.probe_surface",
        {"min_x": cx - 2, "min_z": cz + 2, "max_x": cx + 2, "max_z": cz + 3, "max_y": int(py)},
    )["columns"]
    base_cols = [c for c in surface if c["loaded"] and c["top_y"] == int(py) - 1][:3]
    if len(base_cols) < 3:
        raise ScenarioError(f"S5: insufficient flat columns: {surface}")

    # Place 3 oak_planks blocks on the scanned flat ground.
    equip_item_on_hotbar(mcp, "minecraft:oak_planks")
    placed_planks: list[dict[str, int]] = []
    for col in base_cols:
        mcp.call(
            "input.look_at",
            {"x": col["x"] + 0.5, "y": col["top_y"] + 1.0, "z": col["z"] + 0.5},
        )
        target = mcp.call("world.crosshair_target")
        if target.get("kind") != "block" or target["block"]["face"] != "up":
            raise ScenarioError(f"S5: expected up face on floor, got {target}")
        dest = target["block"]["adjacent_pos"]
        mcp.call("input.mouse_tap", {"button": "right"})
        mcp.call("sim.wait_ticks", {"ticks": 3})
        placed_planks.append(dest)

    # Place oak_sign on top of the center plank and write 4 lines via net.update_sign.
    equip_item_on_hotbar(mcp, "minecraft:oak_sign")
    center = placed_planks[1]
    mcp.call(
        "input.look_at",
        {"x": center["x"] + 0.5, "y": center["y"] + 1.0, "z": center["z"] + 0.5},
    )
    target = mcp.call("world.crosshair_target")
    sign_pos = target["block"]["adjacent_pos"]
    mcp.call("input.mouse_tap", {"button": "right"})
    mcp.call("sim.wait_ticks", {"ticks": 4})
    mcp.call(
        "net.update_sign",
        {
            "x": sign_pos["x"],
            "y": sign_pos["y"],
            "z": sign_pos["z"],
            "is_front_text": True,
            "lines": ["BBB MCP", "Agent Built", "Shelter OK", "26.1"],
        },
    )
    mcp.call("sim.wait_ticks", {"ticks": 3})
    close_screen(mcp)
    shot = take_shot(mcp, out_dir, "s5_build_sign")
    return {"placed_planks": placed_planks, "sign_pos": sign_pos, "screenshot": shot}


def judge_s5(admin: ServerAdmin) -> dict[str, Any]:
    positions = [{"x": x, "y": y, "z": z} for y in (64, 65) for z in range(1, 5) for x in range(-3, 4)]
    blocks = admin.blocks(positions)
    planks = sum(1 for b in blocks if (b.get("state") or {}).get("name") == "minecraft:oak_planks")
    signs = sum(
        1
        for b in blocks
        if (b.get("state") or {}).get("name") in {"minecraft:oak_sign", "minecraft:oak_wall_sign"}
    )
    if planks < 3 or signs < 1:
        raise ScenarioError(f"S5 judge failed: planks={planks}, signs={signs}")
    return {"oak_planks_placed": planks, "oak_signs_placed": signs}


def setup_s6(admin: ServerAdmin) -> None:
    reset_arena(admin)
    admin.cmds(
        f"item replace entity {USERNAME} hotbar.0 with minecraft:diamond_sword 1",
        f"damage {USERNAME} 16",
        # Lit soul campfire kills the low-health player without burning the dropped diamond sword.
        "setblock 0 64 3 minecraft:soul_campfire[lit=true]",
    )


def policy_s6(mcp: McpSession, out_dir: Path) -> dict[str, Any]:
    ensure_gameplay_ready(mcp)
    # 1. Open pause screen via escape, verify cursor_captured drops, then recover via escape.
    mcp.call("input.key", {"key": "escape", "pressed": True})
    mcp.call("input.key", {"key": "escape", "pressed": False})
    mcp.call("sim.wait_ticks", {"ticks": 2})
    paused = mcp.call("app.status")["app"]
    if paused.get("cursor_captured"):
        raise ScenarioError(f"S6: expected cursor_captured=false on pause screen: {paused}")
    close_screen(mcp)

    # 2. Walk onto the soul campfire hazard until death_screen appears (real survival death).
    hazards = scan_local_blocks(
        mcp, radius_xz=6, dy_min=0, dy_max=0, target_names={"minecraft:soul_campfire"}
    )
    if not hazards:
        raise ScenarioError("S6: soul_campfire hazard not found")
    hz = hazards[0]
    walk_to_xz(mcp, hz["x"] + 0.5, hz["z"] + 0.5, stop_dist=0.35, max_steps=10)
    died = False
    for _ in range(25):
        app = mcp.call("app.status")["app"]
        health = (mcp.call("world.client_local_player").get("health") or {}).get("health", 20.0)
        if app.get("death_screen") is not None or float(health) <= 0.0:
            died = True
            break
        mcp.call("sim.wait_ticks", {"ticks": 12})
    if not died:
        raise ScenarioError("S6: player did not die on soul_campfire hazard")

    # 3. Respawn via net.perform_respawn, re-capture cursor, and recover dropped diamond_sword.
    mcp.call("net.perform_respawn")
    for _ in range(10):
        mcp.call("sim.wait_ticks", {"ticks": 6})
        app = mcp.call("app.status")["app"]
        health = (mcp.call("world.client_local_player").get("health") or {}).get("health", 0.0)
        if app.get("death_screen") is None and float(health) > 0.0:
            break
    ensure_gameplay_ready(mcp)
    recovered = collect_dropped_items(mcp, "minecraft:diamond_sword", min_count=1, max_attempts=10)
    equip_item_on_hotbar(mcp, "minecraft:diamond_sword")
    # Step back from the campfire to confirm full playable movement post-respawn.
    mcp.call("input.look", {"yaw": 180.0, "pitch": 0.0})
    mcp.call("input.key", {"key": "forward", "pressed": True, "hold_ticks": 8})
    shot = take_shot(mcp, out_dir, "s6_respawn_recover")
    return {
        "screen_recovered": True,
        "respawned": True,
        "diamond_sword_recovered": recovered,
        "screenshot": shot,
    }


def judge_s6(admin: ServerAdmin) -> dict[str, Any]:
    alive = admin.cmd(f"execute if entity @a[name={USERNAME}]")["result"]
    if alive != 1:
        raise ScenarioError(f"S6 judge failed: player not present/alive ({alive})")
    sword = admin.cmd(f"execute if items entity {USERNAME} container.* minecraft:diamond_sword")[
        "result"
    ]
    if not sword or sword < 1:
        raise ScenarioError(f"S6 judge failed: diamond_sword not recovered ({sword})")
    return {"player_alive_after_respawn": True, "diamond_sword_recovered": sword}


SCENARIOS = {
    "S1": (setup_s1, policy_s1, judge_s1),
    "S2": (setup_s2, policy_s2, judge_s2),
    "S3": (setup_s3, policy_s3, judge_s3),
    "S4": (setup_s4, policy_s4, judge_s4),
    "S5": (setup_s5, policy_s5, judge_s5),
    "S6": (setup_s6, policy_s6, judge_s6),
}


# --- Process Management (Server + Native/Web Client) -------------------------


def wait_server_ready(proc: subprocess.Popen[Any], log_path: Path, timeout: float = 90.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            tail = log_path.read_text(encoding="utf-8", errors="replace")[-3000:]
            raise ScenarioError(f"bbb-server exited {proc.returncode}:\n{tail}")
        if log_path.is_file():
            text = log_path.read_text(encoding="utf-8", errors="replace")
            if '"event":"server.ready"' in text or "server.ready" in text:
                return
        time.sleep(0.15)
    raise ScenarioError(f"bbb-server not ready within {timeout}s; see {log_path}")


def wait_client_mcp_ready(
    proc: subprocess.Popen[Any],
    log_path: Path,
    mcp_port: int,
    *,
    timeout: float = 90.0,
) -> McpSession:
    deadline = time.monotonic() + timeout
    last_err: Exception | None = None
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            tail = log_path.read_text(encoding="utf-8", errors="replace")[-3000:]
            raise ScenarioError(f"client process exited {proc.returncode}:\n{tail}")
        try:
            session = McpSession("127.0.0.1", mcp_port, record_trace=True)
            session.initialize()
            # Wait until client is connected in Play with local player pose initialized.
            counters = session.call("net.counters")
            lp = session.call("world.client_local_player")
            if counters.get("connected") and lp.get("pose") is not None:
                return session
        except Exception as err:
            last_err = err
        time.sleep(0.25)
    raise ScenarioError(f"client MCP not ready in Play within {timeout}s: {last_err}")


def build_web_chrome_args(
    chrome_bin: str,
    url: str,
    profile_dir: Path,
    software_gpu: bool,
    *,
    platform: str | None = None,
) -> list[str]:
    chrome_args = [
        chrome_bin,
        "--headless=new",
        "--no-sandbox",
        "--disable-gpu-sandbox",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        "--disable-renderer-backgrounding",
        "--disable-features=CalculateNativeWinOcclusion",
        "--enable-unsafe-webgpu",
        "--autoplay-policy=no-user-gesture-required",
        "--window-size=854,480",
        f"--user-data-dir={profile_dir}",
    ]
    resolved_platform = platform or sys.platform
    if resolved_platform.startswith("linux"):
        chrome_args.extend(
            [
                "--disable-dev-shm-usage",
                "--enable-features=Vulkan,UseSkiaRenderer",
                "--use-angle=vulkan",
            ]
        )
    if software_gpu:
        chrome_args.extend(
            [
                "--use-webgpu-adapter=swiftshader",
                "--enable-unsafe-swiftshader",
                "--enable-webgpu-developer-features",
            ]
        )
    chrome_args.append(url)
    return chrome_args


def build_native_client_launch(
    client_bin: Path,
    srv_port: int,
    vanilla_root: Path,
    game_dir: Path,
    mcp_port: int,
    software_gpu: bool,
    *,
    display: Any | None = None,
    platform: str | None = None,
    base_env: Mapping[str, str] | None = None,
) -> tuple[list[str], dict[str, str]]:
    client_cmd = [
        str(client_bin),
        "--server",
        f"127.0.0.1:{srv_port}",
        "--username",
        USERNAME,
        "--vanilla-root",
        str(vanilla_root),
        "--game-dir",
        str(game_dir),
        "--mcp-bind",
        f"127.0.0.1:{mcp_port}",
        "--gui-scale",
        "2",
        "--render-distance",
        "6",
    ]
    source_env = os.environ if base_env is None else base_env
    if display is not None and getattr(display, "runtime", None) and getattr(display, "socket", None):
        env = {k: v for k, v in source_env.items() if k != "DISPLAY"}
        env["XDG_RUNTIME_DIR"] = str(display.runtime)
        env["WAYLAND_DISPLAY"] = str(display.socket)
    else:
        env = dict(source_env)
    env["ALSOFT_DRIVERS"] = "null"
    if software_gpu:
        env["BBB_SOFTWARE_GPU"] = "1"
        if (platform or sys.platform).startswith("linux"):
            env.setdefault("LIBGL_ALWAYS_SOFTWARE", "1")
    return client_cmd, env


def start_native_display(
    work_dir: Path,
    *,
    platform: str | None = None,
    environ: Mapping[str, str] | None = None,
) -> Any:
    import client_replay

    resolved_platform = platform or sys.platform
    if not resolved_platform.startswith("linux"):
        return client_replay.NativeDisplay()
    if shutil.which("weston") is not None:
        return client_replay.display_session(work_dir)
    env = os.environ if environ is None else environ
    if env.get("WAYLAND_DISPLAY") or env.get("DISPLAY"):
        return client_replay.NativeDisplay()
    raise ScenarioError(
        "headless Linux native client requires weston on PATH (or an active DISPLAY/WAYLAND_DISPLAY)"
    )


def launch_web_client(
    bin_dir: Path,
    work_dir: Path,
    srv_port: int,
    mcp_port: int,
    vanilla_root: Path,
    wasm_dir: Path,
    software_gpu: bool,
    procs: list[subprocess.Popen[Any]],
) -> tuple[subprocess.Popen[Any], Path]:
    import web_playable_gate as wpg

    host_bin = bin_dir / f"bbb-web-host{EXE_SUFFIX}"
    http_port = free_port()
    ctrl_port = free_port()
    host_log = work_dir / "bbb-web-host.log"
    chrome_log = work_dir / "chrome.log"
    cache_dir = shared_default_web_cache_dir()

    with host_log.open("wb") as out:
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
                str(cache_dir),
                "--control",
                f"127.0.0.1:{ctrl_port}",
                "--mcp-bind",
                f"127.0.0.1:{mcp_port}",
            ],
            stdin=subprocess.DEVNULL,
            stdout=out,
            stderr=subprocess.STDOUT,
        )
        procs.append(host_proc)

    wpg.wait_for_http_ready(http_port, host_proc, host_log)
    chrome_bin = wpg.find_chrome_binary()
    sw = "&software_gpu=1" if software_gpu else ""
    url = (
        f"http://127.0.0.1:{http_port}/"
        f"?control=1&username={USERNAME}&render_distance=6&gui_scale=2"
        f"&opt.pauseOnLostFocus=false&server=localhost:{srv_port}{sw}"
    )
    chrome_args = build_web_chrome_args(
        chrome_bin,
        url,
        work_dir / "chrome-profile",
        software_gpu,
    )
    with chrome_log.open("wb") as out:
        chrome_proc = subprocess.Popen(
            chrome_args, stdin=subprocess.DEVNULL, stdout=out, stderr=subprocess.STDOUT
        )
        procs.append(chrome_proc)
    return host_proc, host_log


def start_session(
    args: argparse.Namespace, work_dir: Path
) -> tuple[ServerAdmin, McpSession, int, list[subprocess.Popen[Any]], Any | None]:
    bin_dir = args.target.resolve()
    server_bin = bin_dir / f"bbb-server{EXE_SUFFIX}"
    client_bin = bin_dir / f"bbb-client{EXE_SUFFIX}"
    for path in (server_bin,):
        if not path.is_file():
            raise ScenarioError(f"missing binary: {path}")

    srv_port = free_port()
    srv_mcp_port = free_port()
    client_mcp_port = free_port()
    srv_log = work_dir / "bbb-server.log"
    procs: list[subprocess.Popen[Any]] = []
    admin: ServerAdmin | None = None
    display: Any | None = None

    try:
        with srv_log.open("wb") as out:
            srv_proc = subprocess.Popen(
                [
                    str(server_bin),
                    "--bind",
                    f"127.0.0.1:{srv_port}",
                    "--mcp-bind",
                    f"127.0.0.1:{srv_mcp_port}",
                    "--mcp-loopback-scope",
                    "command",
                    "--gamemode",
                    "survival",
                    "--difficulty",
                    "normal",
                    "--level-type",
                    "flat",
                    "--view-distance",
                    "6",
                    "--simulation-distance",
                    "6",
                    "--pause-when-empty-seconds",
                    "0",
                    "--world",
                    str(work_dir / "world"),
                ],
                env={**os.environ, "RUST_LOG": "bbb_server=info"},
                stdin=subprocess.DEVNULL,
                stdout=out,
                stderr=subprocess.STDOUT,
            )
            procs.append(srv_proc)
        wait_server_ready(srv_proc, srv_log)
        admin = ServerAdmin(srv_mcp_port)

        if args.client == "web":
            watch_proc, watch_log = launch_web_client(
                bin_dir,
                work_dir,
                srv_port,
                client_mcp_port,
                args.vanilla_root,
                args.wasm_dir,
                args.software_gpu,
                procs,
            )
        else:
            if not client_bin.is_file():
                raise ScenarioError(f"missing binary: {client_bin}")
            display = start_native_display(work_dir)
            game_dir = work_dir / "game-dir"
            game_dir.mkdir(parents=True, exist_ok=True)
            (game_dir / "options.txt").write_text("pauseOnLostFocus:false\n", encoding="utf-8")
            client_log = work_dir / "bbb-client.log"
            client_cmd, env = build_native_client_launch(
                client_bin,
                srv_port,
                args.vanilla_root,
                game_dir,
                client_mcp_port,
                args.software_gpu,
                display=display,
            )
            with client_log.open("wb") as out:
                client_proc = subprocess.Popen(
                    client_cmd,
                    env=env,
                    stdin=subprocess.DEVNULL,
                    stdout=out,
                    stderr=subprocess.STDOUT,
                )
                procs.append(client_proc)
            watch_proc, watch_log = client_proc, client_log

        client_mcp = wait_client_mcp_ready(watch_proc, watch_log, client_mcp_port, timeout=120.0)
        return admin, client_mcp, client_mcp_port, procs, display
    except BaseException:
        stop_processes(admin, None, procs, display)
        raise


def stop_processes(
    admin: ServerAdmin | None,
    client_mcp: McpSession | None,
    procs: list[subprocess.Popen[Any]],
    display: Any | None = None,
) -> None:
    if client_mcp is not None:
        try:
            client_mcp.call("app.quit")
        except Exception:
            pass
    if admin is not None:
        admin.save_and_stop()
    deadline = time.monotonic() + 8.0
    for proc in reversed(procs):
        remaining = max(0.1, deadline - time.monotonic())
        try:
            proc.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            terminate_process_tree(proc, timeout=3.0)
    if display is not None:
        try:
            display.close()
        except Exception:
            pass


def run_policy_mode(args: argparse.Namespace) -> int:
    out_dir = args.out.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    selected = [s.strip().upper() for s in args.scenarios.split(",") if s.strip()]
    for name in selected:
        if name not in SCENARIOS:
            raise ScenarioError(f"unknown scenario: {name} (expected subset of {ALL_SCENARIOS})")

    admin: ServerAdmin | None = None
    client_mcp: McpSession | None = None
    procs: list[subprocess.Popen[Any]] = []
    display: Any | None = None
    report: dict[str, Any] = {
        "client": args.client,
        "software_gpu": args.software_gpu,
        "scenarios": {},
    }
    try:
        admin, client_mcp, _, procs, display = start_session(args, out_dir)
        tools = client_mcp.list_tools()
        tool_names = {t["name"] for t in tools}
        required_new = {
            "world.probe_blocks",
            "world.probe_surface",
            "world.crosshair_target",
            "world.nearby_entities",
            "input.look",
            "input.look_at",
            "input.scroll",
            "sim.wait_ticks",
        }
        missing = sorted(required_new - tool_names)
        if missing:
            raise ScenarioError(f"client MCP missing expected new tools: {missing}")
        report["tool_count"] = len(tools)
        report["instructions"] = client_mcp.instructions

        all_ok = True
        for name in selected:
            setup_fn, policy_fn, judge_fn = SCENARIOS[name]
            calls_before = client_mcp.metrics.calls
            errs_before = client_mcp.metrics.errors
            t0 = time.monotonic()
            try:
                ensure_gameplay_ready(client_mcp)
                setup_fn(admin)
                client_mcp.call("sim.wait_ticks", {"ticks": 6})
                policy_result = policy_fn(client_mcp, out_dir)
                client_mcp.call("sim.wait_ticks", {"ticks": 4})
                verdict = judge_fn(admin)
                report["scenarios"][name] = {
                    "passed": True,
                    "elapsed_s": round(time.monotonic() - t0, 2),
                    "mcp_calls": client_mcp.metrics.calls - calls_before,
                    "mcp_errors": client_mcp.metrics.errors - errs_before,
                    "policy": policy_result,
                    "verdict": verdict,
                }
                print(
                    f"[PASS] {name} ({report['scenarios'][name]['elapsed_s']}s, "
                    f"{report['scenarios'][name]['mcp_calls']} MCP calls)",
                    flush=True,
                )
            except Exception as err:
                all_ok = False
                report["scenarios"][name] = {
                    "passed": False,
                    "elapsed_s": round(time.monotonic() - t0, 2),
                    "mcp_calls": client_mcp.metrics.calls - calls_before,
                    "mcp_errors": client_mcp.metrics.errors - errs_before,
                    "error": str(err),
                }
                print(f"[FAIL] {name}: {err}", file=sys.stderr, flush=True)
        (out_dir / "actions.jsonl").write_text(
            "\n".join(json.dumps(row) for row in client_mcp.metrics.trace) + "\n",
            encoding="utf-8",
        )
        (out_dir / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"Report written to {out_dir / 'report.json'}", flush=True)
        return 0 if all_ok else 1
    finally:
        stop_processes(admin, client_mcp, procs, display)


def run_serve_mode(args: argparse.Namespace) -> int:
    out_dir = args.out.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    name = args.scenarios.split(",")[0].strip().upper()
    if name not in SCENARIOS:
        raise ScenarioError(f"unknown scenario: {name}")
    setup_fn, _, judge_fn = SCENARIOS[name]
    ready_file = Path(args.ready_file).resolve()
    done_file = Path(args.done_file).resolve()
    ready_file.unlink(missing_ok=True)
    done_file.unlink(missing_ok=True)

    admin: ServerAdmin | None = None
    client_mcp: McpSession | None = None
    procs: list[subprocess.Popen[Any]] = []
    display: Any | None = None
    try:
        admin, client_mcp, client_mcp_port, procs, display = start_session(args, out_dir)
        setup_fn(admin)
        client_mcp.call("sim.wait_ticks", {"ticks": 6})
        # Publish ONLY client MCP details and the natural-language mission (never server MCP!).
        info = {
            "scenario": name,
            "client": args.client,
            "client_mcp_url": f"http://127.0.0.1:{client_mcp_port}/mcp",
            "client_mcp_port": client_mcp_port,
            "mission": SCENARIO_MISSIONS[name],
            "shots_dir": str(out_dir / "shots"),
        }
        ready_file.write_text(json.dumps(info, indent=2) + "\n", encoding="utf-8")
        print(f"SERVE READY {name}: {info['client_mcp_url']}", flush=True)

        deadline = time.monotonic() + float(args.timeout_seconds)
        while time.monotonic() < deadline and not done_file.exists():
            time.sleep(0.25)

        verdict_doc: dict[str, Any] = {"scenario": name, "timed_out": not done_file.exists()}
        try:
            verdict_doc["verdict"] = judge_fn(admin)
            verdict_doc["passed"] = True
        except Exception as err:
            verdict_doc["passed"] = False
            verdict_doc["error"] = str(err)
        (out_dir / "verdict.json").write_text(
            json.dumps(verdict_doc, indent=2) + "\n", encoding="utf-8"
        )
        print(json.dumps(verdict_doc, indent=2), flush=True)
        return 0 if verdict_doc["passed"] else 1
    finally:
        stop_processes(admin, client_mcp, procs, display)


def run_call_mode(args: argparse.Namespace) -> int:
    parsed = urllib.request.urlparse(args.url)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or 80
    session = McpSession(host, port, client_name="mcp-cli", timeout=20.0)
    init = session.initialize()
    if args.tool == "__info__":
        print(
            json.dumps(
                {"instructions": init.get("instructions"), "tools": session.list_tools()},
                indent=2,
            )
        )
        return 0
    raw_args = sys.stdin.read() if args.args_json == "-" else args.args_json
    tool_args = json.loads(raw_args) if raw_args and raw_args.strip() else {}
    res = session.call_raw(args.tool, tool_args)
    # Strip raw base64 image blobs from stdout when saving screenshots so LLM context stays clean.
    if not res.get("isError") and "structuredContent" in res:
        print(json.dumps(res["structuredContent"], indent=2))
    else:
        print(json.dumps(res, indent=2))
    return 1 if res.get("isError") else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("policy", "serve", "call"), default="policy")
    parser.add_argument("--client", choices=("native", "web"), default="native")
    parser.add_argument("--scenarios", default=",".join(ALL_SCENARIOS))
    parser.add_argument("--target", type=Path, default=default_target_dir())
    parser.add_argument("--vanilla-root", type=Path, default=default_vanilla_root())
    parser.add_argument("--wasm-dir", type=Path, default=default_wasm_dir())
    parser.add_argument(
        "--out",
        type=Path,
        default=Path(tempfile.gettempdir()) / f"bbb-mcp-scenarios-{uuid.uuid4().hex[:8]}",
    )
    parser.add_argument(
        "--software-gpu",
        action="store_true",
        default=os.environ.get("BBB_SOFTWARE_GPU") == "1",
    )
    parser.add_argument("--ready-file", default="")
    parser.add_argument("--done-file", default="")
    parser.add_argument("--timeout-seconds", type=float, default=600.0)
    parser.add_argument("--url", default="")
    parser.add_argument("--tool", default="")
    parser.add_argument("--args-json", default="{}")
    args = parser.parse_args()

    if args.mode == "call":
        return run_call_mode(args)
    if args.mode == "serve":
        if not args.ready_file or not args.done_file:
            parser.error("--mode serve requires --ready-file and --done-file")
        return run_serve_mode(args)
    return run_policy_mode(args)


if __name__ == "__main__":
    raise SystemExit(main())
