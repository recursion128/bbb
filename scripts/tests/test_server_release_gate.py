"""Release timing only covers the ticks that performed the requested burst."""

from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock

import pytest
import server_release_gate as gate


def sample(tick, micros):
    return {"event": "world.tick_sample", "tick": tick, "tick_micros": micros, "connections": 20}


def test_burst_health_excludes_startup_and_post_generation_idle_ticks():
    records = [sample(1, 2_000_000)]
    records += [sample(tick, 1000) for tick in range(2, 102)]
    records += [sample(102, 2_000_000)]
    server = SimpleNamespace(events=Mock(return_value=records))
    health = gate.burst_tick_health(server, 1, 101)
    assert health["ticks"] == 100
    assert health["max_tick_micros"] == health["p99_tick_micros"] == 1000
    assert health["ticks_over_period"] == 0


def test_post_burst_idle_ticks_cannot_dilute_a_failing_percentile():
    records = [sample(1, 1000), sample(2, 50_000)]
    records += [sample(tick, 1000) for tick in range(3, 1003)]
    server = SimpleNamespace(events=Mock(return_value=records))
    health = gate.burst_tick_health(server, 0, 2)
    assert health["p99_tick_micros"] == 50_000
    with pytest.raises(gate.ReleaseGateError, match="burst p99"):
        gate.check_burst_health(health)


def test_missing_tick_rejects_the_burst_sample():
    server = SimpleNamespace(events=Mock(return_value=[sample(1, 1000), sample(3, 1000)]))
    with pytest.raises(gate.ReleaseGateError, match="incomplete"):
        gate.burst_tick_health(server, 0, 3)


def test_departed_players_must_disappear_before_reconnecting(monkeypatch):
    players = Mock(side_effect=[["departing"], []])
    monkeypatch.setattr(gate, "connected_players", players)
    monkeypatch.setattr(gate.time, "sleep", lambda _: None)
    assert gate.wait_for_player_count(Mock(), 0, timeout=1) == []


def test_local_bot_scenarios_raise_max_connections_per_ip():
    assert gate.local_bot_connection_args() == ["--max-connections-per-ip", "24"]
    assert gate.local_bot_connection_args(8) == ["--max-connections-per-ip", "16"]
    source = (Path(__file__).resolve().parents[1] / "server_release_gate.py").read_text(
        encoding="utf-8"
    )
    assert "server.start(extra=local_bot_connection_args())" in source
    assert "*local_bot_connection_args()" in source


def test_list_world_files_uses_posix_separators(tmp_path):
    nested = tmp_path / "data" / "minecraft" / "world_gen_settings.dat"
    nested.parent.mkdir(parents=True)
    nested.write_bytes(b"ok")
    (tmp_path / "level.dat").write_bytes(b"ok")
    assert gate.list_world_files(tmp_path) == [
        "data/minecraft/world_gen_settings.dat",
        "level.dat",
    ]


def test_hard_kill_server_supports_posix_sigkill_and_windows_kill(monkeypatch):
    posix_server = Mock()
    posix_server.wait_exit.return_value = -9
    monkeypatch.setattr(gate.signal, "SIGKILL", 9, raising=False)
    assert gate.hard_kill_server(posix_server) == -9
    posix_server.signal.assert_called_once_with(9)

    win_server = Mock()
    win_server.wait_exit.return_value = 1
    monkeypatch.delattr(gate.signal, "SIGKILL", raising=False)
    assert gate.hard_kill_server(win_server) == 1
    win_server.kill.assert_called_once_with()
    win_server.signal.assert_not_called()

    zero_exit_server = Mock()
    zero_exit_server.wait_exit.return_value = 0
    with pytest.raises(gate.ReleaseGateError, match="hard kill"):
        gate.hard_kill_server(zero_exit_server)


def test_signal_family_and_posix_world_capture_fail_cleanly_on_windows(monkeypatch, tmp_path):
    monkeypatch.setattr(gate.os, "name", "nt")
    paths = gate.Paths(
        artifact_dir=tmp_path,
        server_bin=tmp_path / "bbb-server.exe",
        client_bin=tmp_path / "bbb-client.exe",
        admin_bin=tmp_path / "bbb-admin.exe",
        world_template=tmp_path,
    )
    with pytest.raises(gate.ReleaseGateError, match="unsupported on Windows"):
        gate.mode_signal(paths)

    win_error = gate._official_capture_host_error(platform_os="nt")
    assert win_error is not None and "Windows" in win_error
    missing_sh = gate._official_capture_host_error(platform_os="posix", sh_bin="")
    assert missing_sh is not None and "sh" in missing_sh
    assert gate._official_capture_host_error(platform_os="posix", sh_bin="/bin/sh") is None
