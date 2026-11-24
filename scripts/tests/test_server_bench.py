"""Benchmark commands must not repeat mutations after an admin timeout."""

from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch

import pytest
import server_bench as bench
from server_release_gate import ReleaseGateError


def test_timed_out_mutation_is_sent_once_and_only_diagnostics_are_retried():
    timeout = ReleaseGateError("the management server did not answer in time")
    server = SimpleNamespace(management=Mock(side_effect=[timeout, timeout, {}]))
    assert bench.command(server, "summon minecraft:cow 0 64 0") is None
    calls = server.management.call_args_list
    assert len(calls) == 3
    assert calls[0].args[:2] == ("call", "command.execute")
    assert [call.args for call in calls[1:]] == [("call", "server.diagnostics")] * 2


def test_non_timeout_failure_is_not_retried():
    server = SimpleNamespace(management=Mock(side_effect=ReleaseGateError("connection refused")))
    with pytest.raises(ReleaseGateError, match="connection refused"):
        bench.command(server, "summon minecraft:cow 0 64 0")
    assert server.management.call_count == 1


def test_terrain_snapshot_keeps_generated_data_and_ignores_tick_time(tmp_path: Path):
    chunk = {
        "xPos": 125,
        "zPos": 125,
        "sections": [{"Y": -4, "block_states": {"palette": [{"Name": "minecraft:stone"}]}}],
        "Heightmaps": {"WORLD_SURFACE": [1]},
        "structures": {},
        "LastUpdate": 1,
    }
    with patch.object(bench, "read_chunk", return_value=chunk) as read:
        first = tmp_path / "first.json"
        digest = bench.terrain_snapshot(tmp_path, first)
        assert read.call_count == 256
        chunk["LastUpdate"] = 200
        second = tmp_path / "second.json"
        assert bench.terrain_snapshot(tmp_path, second) == digest
        assert second.read_bytes() == first.read_bytes()
        chunk["sections"] = []
        assert bench.terrain_snapshot(tmp_path, second) != digest


@pytest.mark.parametrize("failure", [None, "process", "hold", "population"])
def test_player_timing_requires_full_playable_workload(failure):
    runs = []
    for seat in range(1, 21):
        report = {
            "reached_state": "Play",
            "first_chunk": {},
            "first_chunk_summary": {"state": "Decoded"},
            "unsupported_packets": 0,
            "hold": {"completed": failure != "hold" or seat != 1},
        }
        runs.append(SimpleNamespace(
            username=f"bench-{seat:02d}",
            wait=Mock(return_value=1 if failure == "process" and seat == 1 else 0),
            report=Mock(return_value=report),
            failure_detail=Mock(return_value="failed session"),
        ))
    samples = [
        {"mean_tick_micros": 100, "max_tick_micros": 200, "connections": 20}
        for _ in range(3)
    ]
    if failure == "population":
        samples[-1]["connections"] = 19
    with (
        patch.object(bench, "run_round", return_value=runs),
        patch.object(bench, "wait_for_player_count", return_value=[r.username for r in runs]),
        patch.object(bench, "tick_health", return_value=[{}] * 4),
        patch.object(bench, "wait_windows", return_value=samples) as wait,
        patch.object(bench, "process_footprint", return_value={"rss_kib": 123}),
    ):
        if failure:
            with pytest.raises(ReleaseGateError):
                bench.benchmark_players(None, None)
        else:
            result = bench.benchmark_players(None, None)
            assert result["windows"] == 3
            assert result["completed"] == result["peak_connections"] == 20
            assert result["mean_tick_micros"] == 100
        wait.assert_called_once_with(None, 5, 3)


def test_cross_platform_process_footprint_and_cpu_model_work_on_host():
    import os

    import client_replay as replay

    server = SimpleNamespace(name="bench", process=SimpleNamespace(pid=os.getpid()))
    footprint = bench.process_footprint(server)
    assert footprint["rss_kib"] > 0
    assert replay.peak_rss(os.getpid()) > 0
    assert replay.cpu_seconds(os.getpid()) >= 0.0
    assert isinstance(bench.host_cpu_model(), str)


@pytest.mark.parametrize("has_autosave", [True, False])
def test_fast_flying_autosave_records_mspt_and_requires_autosave(has_autosave):
    run = SimpleNamespace(
        username="bench-flyer",
        wait=Mock(return_value=0),
        report=Mock(
            return_value={
                "reached_state": "Play",
                "first_chunk": {},
                "first_chunk_summary": {"state": "Decoded"},
                "unsupported_packets": 0,
                "hold": {"completed": True},
            }
        ),
        failure_detail=Mock(return_value="failed session"),
    )
    events_calls = 0

    def fake_events():
        nonlocal events_calls
        events_calls += 1
        if has_autosave and events_calls >= 2:
            return [{"event": "world.saved", "trigger": "autosave"}]
        return []

    server = SimpleNamespace(events=fake_events)
    samples = [
        {"mean_tick_micros": 12500, "max_tick_micros": 34000},
        {"mean_tick_micros": 17500, "max_tick_micros": 41000},
    ]
    commands = []
    with (
        patch.object(bench, "run_round", return_value=[run]),
        patch.object(bench, "wait_for_player_count", return_value=["bench-flyer"]),
        patch.object(bench, "command", side_effect=lambda _s, text: commands.append(text)),
        patch.object(bench, "tick_health", side_effect=[[], samples, samples]),
        patch.object(bench, "process_footprint", return_value={"rss_kib": 456}),
        patch.object(bench.time, "sleep"),
    ):
        if has_autosave:
            result = bench.benchmark_fast_flying_autosave(None, server)
            assert result["mean_tick_micros"] == 15000
            assert result["max_tick_micros"] == 41000
            assert result["mean_mspt"] == 15.0
            assert result["max_mspt"] == 41.0
            assert result["autosaves"] == 1
            assert result["hops"] >= 1
            assert "gamemode creative bench-flyer" in commands
            assert "tick rate 1" in commands
            assert "tick rate 20" in commands
            assert any(cmd.startswith("tp bench-flyer ") for cmd in commands)
        else:
            with patch.object(bench.time, "monotonic", side_effect=[0.0, 0.0, 100.0]):
                with pytest.raises(ReleaseGateError):
                    bench.benchmark_fast_flying_autosave(None, server)


def test_parse_scenarios_validates_and_orders_subset():
    assert bench.parse_scenarios("all") == bench.ALL_SCENARIOS
    assert bench.parse_scenarios("fast_flying_autosave,idle") == ("idle", "fast_flying_autosave")
    with pytest.raises(SystemExit):
        bench.parse_scenarios("bogus")
    with pytest.raises(SystemExit):
        bench.parse_scenarios("mobs")


def test_bench_passes_max_connections_per_ip_for_local_players():
    source = (Path(__file__).resolve().parents[1] / "server_bench.py").read_text(encoding="utf-8")
    assert "server.start(extra=local_bot_connection_args())" in source
