#!/usr/bin/env python3
"""Exercise real MCP gameplay and the existing lockstep recorder/player."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
import uuid
from pathlib import Path

import client_replay as replay


def scenario() -> replay.Scenario:
    s = replay.Scenario("mcp_play_loop", replay.JOIN + 250, client_args=["--gui-scale", "2"])
    t = replay.JOIN
    s.server(
        t - 40,
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "time set 6000",
        "weather clear",
        "fill -8 63 -8 8 63 8 minecraft:stone",
        "fill -8 64 -8 8 68 8 minecraft:air",
        f"tp {replay.USERNAME} 0.5 64 0.5 0 0",
    )
    s.control(t, "net.counters").control(t, "world.client_local_player")
    s.key(t + 1, "forward", 20).control(t + 26, "world.client_local_player")
    s.server(
        t + 27, f"execute positioned 0.5 64 0.5 if entity @a[name={replay.USERNAME},distance=1..8]"
    )
    s.server(
        t + 35,
        f"tp {replay.USERNAME} 0.5 64 0.5 0 20",
        "setblock 0 64 2 minecraft:dirt",
        "setblock 0 64 3 minecraft:stone",
        f"item replace entity {replay.USERNAME} hotbar.0 with minecraft:stone 8",
    )
    s.key(t + 45, "hotbar1").control(t + 49, "world.probe_block", x=0, y=64, z=2)
    s.control(t + 50, "input.mouse_tap", button="left")
    s.control(t + 65, "world.probe_block", x=0, y=64, z=2)
    s.control(t + 65, "world.last_block_changed_ack")
    s.server(t + 66, "execute if block 0 64 2 minecraft:air")
    s.control(t + 70, "input.mouse_tap", button="right")
    s.control(t + 85, "world.probe_block", x=0, y=64, z=2)
    s.control(t + 85, "world.last_block_changed_ack")
    s.server(t + 86, "execute if block 0 64 2 minecraft:stone")
    s.server(
        t + 95,
        f"gamemode survival {replay.USERNAME}",
        f"item replace entity {replay.USERNAME} hotbar.1 with minecraft:diamond 3",
    )
    s.key(t + 110, "inventory").control(t + 115, "world.client_inventory")
    s.control(t + 120, "input.click", button="left", **replay.gui_px(159, 187))
    s.control(t + 125, "world.client_inventory")
    s.control(t + 130, "input.click", button="left", **replay.gui_px(141, 129))
    s.control(t + 145, "world.client_inventory")
    s.server(
        t + 146, f"execute if items entity {replay.USERNAME} inventory.0 minecraft:diamond[count=3]"
    )
    s.shot(t + 150, "inventory_moved").key(t + 155, "escape")
    s.control(t + 160, "world.client_inventory").shot(t + 165, "placed")
    return s


def verify_actions(directory: Path) -> dict:
    tools = json.loads((directory / "mcp-tools.json").read_text())["tools"]
    key_tool = next(tool for tool in tools if tool["name"] == "input.key")
    keys = key_tool["inputSchema"]["properties"]["key"]["enum"]
    assert {
        "f1",
        "f3",
        "f4",
        "f5",
        "f6",
        "tab",
        "drop",
        "swap_offhand",
        "advancements",
        "social",
    } <= set(keys), keys
    actions = replay.read_jsonl(directory / "actions.jsonl")

    def results(method: str) -> list:
        return [row["result"] for row in actions if row["method"] == method]

    counters = results("net.counters")[0]
    assert counters["connected"], counters
    before, after = results("world.client_local_player")
    p, q = before["pose"]["position"], after["pose"]["position"]
    assert q["z"] > p["z"] + 1.0, (p, q)
    blocks = results("world.probe_block")
    assert [block["block_name"] for block in blocks] == [
        "minecraft:dirt",
        "minecraft:air",
        "minecraft:stone",
    ], blocks
    acknowledgements = results("world.last_block_changed_ack")
    assert all(value is not None for value in acknowledgements), acknowledgements
    assert acknowledgements[1]["sequence"] > acknowledgements[0]["sequence"], acknowledgements
    opened, carried, moved, closed = results("world.client_inventory")

    def slot(inventory: dict, index: int) -> dict:
        return next(
            row["item"] for row in inventory["inventory_menu"]["slots"] if row["slot"] == index
        )

    original = slot(opened, 37)
    assert opened["local_inventory_open"] and original["count"] == 3, opened
    assert carried["cursor_item"]["count"] == 3 and slot(carried, 37)["count"] == 0, carried
    destination = slot(moved, 9)
    assert destination["count"] == 3 and destination["item_id"] == original["item_id"], moved
    assert moved["cursor_item"]["count"] == 0 and slot(moved, 37)["count"] == 0, moved
    assert not closed["local_inventory_open"], closed
    return {
        "connected": True,
        "before": p,
        "after": q,
        "block_acknowledgements": acknowledgements,
        "moved_item": {
            "item_id": destination["item_id"],
            "count": destination["count"],
            "from_slot": 37,
            "to_slot": 9,
        },
    }


def verify_server(capture: Path) -> None:
    commands = replay.read_jsonl(capture / "commands.jsonl")
    checked = 0
    for row in commands:
        if row["command"].startswith("execute "):
            expected = 3 if "if items" in row["command"] else 1
            assert row["answer"]["command"]["result"] == expected, row
            checked += 1
    assert checked == 4, checked


def verify_images(tools: replay.Tools, directory: Path) -> None:
    paths = sorted((directory / "shots").glob("*.png"))
    if (directory / "recovered.png").exists():
        paths.append(directory / "recovered.png")
    assert paths, directory
    jobs = [
        {"left": str(path), "right": str(path), "diff": str(path.with_suffix(".diff.png"))}
        for path in paths
    ]
    result = subprocess.run(
        [str(tools.bin("bbb-replay-diff"))],
        input=json.dumps(jobs),
        text=True,
        capture_output=True,
        check=True,
    )
    decoded = [json.loads(line) for line in result.stdout.splitlines()]
    assert len(decoded) == len(paths) and all(
        row["pixels"] == replay.WIDTH * replay.HEIGHT for row in decoded
    ), decoded


def verify_screenshot_failure(tools: replay.Tools, output: Path) -> None:
    output.mkdir()
    blocker = output / "blocker"
    blocker.write_text("ordinary file, not a directory\n")
    weston = replay.display_session(output)
    link = replay.Link(None)
    client = None
    try:
        plan = replay.Scenario("mcp_screenshot_failure", 1)
        client = replay.Client(tools, output, weston, link.port, plan)
        client.wait_control()
        link.accept()
        result, _ = client._mcp_request(
            "tools/call",
            {"name": "renderer.screenshot", "arguments": {"path": str(blocker / "failure.png")}},
        )
        assert result["isError"] is True, result
        (output / "failure.json").write_text(json.dumps(result, indent=2) + "\n")
        assert client.call("app.status")["app"]["running"]
        path = output / "recovered.png"
        result = client.call("renderer.screenshot", {"path": str(path)})
        replay.Session(plan, client, link, output).completed_screenshot(path, result)
        verify_images(tools, output)
    finally:
        if client is not None:
            client.quit()
        link.close()
        weston.close()


def run(
    target: Path,
    output: Path,
    *,
    software_gpu: bool = False,
    vanilla_root: Path | None = None,
) -> None:
    output = output.resolve()
    if output.exists():
        raise replay.ReplayError(f"output already exists: {output}")
    output.mkdir(parents=True)
    tools = replay.Tools(
        target.resolve(),
        "mcp",
        software_gpu=software_gpu,
        vanilla_root=vanilla_root,
    )
    verify_screenshot_failure(tools, output / "screenshot-failure")
    plan = scenario()
    capture = replay.record(tools, plan)
    recorded = output / "record"
    shutil.copytree(capture, recorded)
    record = verify_actions(recorded)
    verify_images(tools, recorded)
    verify_server(recorded)
    checks = {"record": record, "capture": str(recorded), "transport": "mcp"}
    for name in ("replay-a", "replay-b"):
        destination = output / name
        replay.replay(tools, plan, destination)
        checks[name] = verify_actions(destination)
        verify_images(tools, destination)
    report = replay.compare_scenario(
        tools, plan, output / "replay-a", output / "replay-b", output / "diff"
    )
    assert replay.passes(report, 0, 0), report
    checks["comparison"] = report
    checks["video_recording"] = False
    (output / "report.json").write_text(json.dumps(checks, indent=2) + "\n")
    print(f"MCP playable and recording loop passed: {output / 'report.json'}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", type=Path, default=replay.default_target_debug_dir())
    parser.add_argument(
        "--out",
        type=Path,
        default=Path(tempfile.gettempdir()) / f"bbb-client-e2e-{uuid.uuid4().hex[:8]}",
    )
    parser.add_argument(
        "--software-gpu",
        action="store_true",
        default=os.environ.get("BBB_SOFTWARE_GPU") == "1",
    )
    parser.add_argument("--vanilla-root", type=Path, default=None)
    arguments = parser.parse_args()
    run(
        arguments.target,
        arguments.out,
        software_gpu=arguments.software_gpu,
        vanilla_root=arguments.vanilla_root,
    )


if __name__ == "__main__":
    main()
