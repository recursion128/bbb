"""Unit tests for scripts/mcp_agent_scenarios.py launch and path helpers."""

from __future__ import annotations

import os
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import mcp_agent_scenarios as scenarios  # noqa: E402
from bbb_tools.python_runner import default_target_dir as shared_default_target_dir  # noqa: E402
from bbb_tools.python_runner import default_target_root  # noqa: E402


class NativeClientLaunchTests(unittest.TestCase):
    def test_software_gpu_uses_env_var_never_cli_flag(self) -> None:
        cmd, env = scenarios.build_native_client_launch(
            Path("/bin/bbb-client"),
            25565,
            Path("/vanilla"),
            Path("/work/game-dir"),
            25590,
            software_gpu=True,
            platform="linux",
            base_env={"PATH": "/usr/bin"},
        )
        self.assertNotIn("--software-gpu", cmd)
        self.assertEqual(env["BBB_SOFTWARE_GPU"], "1")
        self.assertEqual(env["LIBGL_ALWAYS_SOFTWARE"], "1")
        self.assertEqual(env["ALSOFT_DRIVERS"], "null")

    def test_hardware_gpu_omits_software_gpu_env(self) -> None:
        cmd, env = scenarios.build_native_client_launch(
            Path("/bin/bbb-client"),
            25565,
            Path("/vanilla"),
            Path("/work/game-dir"),
            25590,
            software_gpu=False,
            platform="win32",
            base_env={"PATH": "C:\\Windows"},
        )
        self.assertNotIn("--software-gpu", cmd)
        self.assertNotIn("BBB_SOFTWARE_GPU", env)
        self.assertNotIn("LIBGL_ALWAYS_SOFTWARE", env)
        self.assertEqual(env["ALSOFT_DRIVERS"], "null")

    def test_wayland_display_session_replaces_x11_display(self) -> None:
        fake_display = SimpleNamespace(runtime="/tmp/xdg-run", socket="bbb-replay-1")
        _cmd, env = scenarios.build_native_client_launch(
            Path("/bin/bbb-client"),
            25565,
            Path("/vanilla"),
            Path("/work/game-dir"),
            25590,
            software_gpu=True,
            display=fake_display,
            platform="linux",
            base_env={"DISPLAY": ":0", "PATH": "/usr/bin"},
        )
        self.assertNotIn("DISPLAY", env)
        self.assertEqual(env["XDG_RUNTIME_DIR"], "/tmp/xdg-run")
        self.assertEqual(env["WAYLAND_DISPLAY"], "bbb-replay-1")


class WebChromeArgsTests(unittest.TestCase):
    def test_linux_software_gpu_includes_vulkan_and_swiftshader_flags(self) -> None:
        url = "http://127.0.0.1:8080/?software_gpu=1"
        args = scenarios.build_web_chrome_args(
            "/usr/bin/chromium",
            url,
            Path("/work/chrome-profile"),
            software_gpu=True,
            platform="linux",
        )
        for expected in (
            "--disable-dev-shm-usage",
            "--enable-features=Vulkan,UseSkiaRenderer",
            "--use-angle=vulkan",
            "--use-webgpu-adapter=swiftshader",
            "--enable-unsafe-swiftshader",
            "--enable-webgpu-developer-features",
        ):
            self.assertIn(expected, args)
        self.assertEqual(args[-1], url)

    def test_non_linux_hardware_gpu_omits_linux_vulkan_and_swiftshader_flags(self) -> None:
        url = "http://127.0.0.1:8080/"
        args = scenarios.build_web_chrome_args(
            "chrome.exe",
            url,
            Path("C:/work/chrome-profile"),
            software_gpu=False,
            platform="win32",
        )
        for omitted in (
            "--disable-dev-shm-usage",
            "--enable-features=Vulkan,UseSkiaRenderer",
            "--use-angle=vulkan",
            "--use-webgpu-adapter=swiftshader",
            "--enable-unsafe-swiftshader",
            "--enable-webgpu-developer-features",
        ):
            self.assertNotIn(omitted, args)
        self.assertEqual(args[-1], url)


class NativeDisplaySelectionTests(unittest.TestCase):
    def test_non_linux_uses_native_display(self) -> None:
        display = scenarios.start_native_display(Path("/tmp/work"), platform="win32")
        self.assertIsNone(display.runtime)
        self.assertIsNone(display.socket)

    def test_linux_starts_weston_when_available(self) -> None:
        sentinel = SimpleNamespace(runtime="/run/user/1000", socket="bbb-weston")
        with (
            patch("mcp_agent_scenarios.shutil.which", return_value="/usr/bin/weston"),
            patch("client_replay.display_session", return_value=sentinel) as mock_session,
        ):
            display = scenarios.start_native_display(Path("/tmp/work"), platform="linux")
        self.assertIs(display, sentinel)
        mock_session.assert_called_once_with(Path("/tmp/work"))

    def test_linux_headless_without_weston_or_display_fails_clearly(self) -> None:
        with patch("mcp_agent_scenarios.shutil.which", return_value=None):
            with self.assertRaisesRegex(scenarios.ScenarioError, "weston"):
                scenarios.start_native_display(Path("/tmp/work"), platform="linux", environ={})

    def test_linux_with_existing_display_falls_back_when_weston_absent(self) -> None:
        with patch("mcp_agent_scenarios.shutil.which", return_value=None):
            display = scenarios.start_native_display(
                Path("/tmp/work"), platform="linux", environ={"DISPLAY": ":0"}
            )
        self.assertIsNone(display.socket)


class PathResolutionTests(unittest.TestCase):
    def test_target_root_and_dir_precedence(self) -> None:
        self.assertEqual(
            default_target_root({"BBB_TARGET_ROOT": "/custom-root"}),
            Path("/custom-root"),
        )
        self.assertEqual(
            shared_default_target_dir(
                environ={"BBB_TARGET_ROOT": "/custom-root", "BBB_CARGO_TARGET_NAME": "lane-2"}
            ),
            Path("/custom-root/bbb-target-lane-2"),
        )
        self.assertEqual(
            shared_default_target_dir(
                environ={
                    "CARGO_TARGET_DIR": "/explicit-target",
                    "BBB_TARGET_ROOT": "/custom-root",
                }
            ),
            Path("/explicit-target"),
        )

    def test_scenario_defaults_honor_target_root_and_vanilla_env(self) -> None:
        env = {
            "BBB_TARGET_ROOT": "/custom-root",
            "BBB_VANILLA_ROOT": "/custom-vanilla",
        }
        with patch.dict(os.environ, env, clear=True):
            self.assertEqual(
                scenarios.default_target_dir(), Path("/custom-root/bbb-target-main/debug")
            )
            self.assertEqual(scenarios.default_wasm_dir(), Path("/custom-root/bbb-web-dist"))
            self.assertEqual(scenarios.default_vanilla_root(), Path("/custom-vanilla").absolute())

    def test_stop_processes_uses_process_tree_termination_on_timeout(self) -> None:
        import subprocess
        from unittest.mock import Mock

        proc = Mock()
        proc.wait.side_effect = subprocess.TimeoutExpired(cmd="chrome", timeout=0.1)
        with patch("mcp_agent_scenarios.terminate_process_tree") as terminate_tree:
            scenarios.stop_processes(None, None, [proc])
        terminate_tree.assert_called_once_with(proc, timeout=3.0)


if __name__ == "__main__":
    unittest.main()
