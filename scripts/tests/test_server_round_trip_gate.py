import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "server-round-trip-gate-26.1.sh"


class RoundTripGateTests(unittest.TestCase):
    def run_gate(self, env):
        if os.name != "posix":
            self.skipTest("POSIX shell harness")
        return subprocess.run(
            ["sh", str(SCRIPT)],
            env={"PATH": "/usr/bin:/bin", **env},
            capture_output=True,
            text=True,
        )

    @unittest.skipUnless(os.name == "posix", "POSIX shell harness")
    def test_shell_syntax(self):
        subprocess.run(["sh", "-n", str(SCRIPT)], check=True)

    def test_usage(self):
        result = self.run_gate({})
        self.assertEqual(2, result.returncode)
        self.assertIn("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA", result.stderr)
        help_result = subprocess.run(
            ["sh", str(SCRIPT), "--help"], capture_output=True, text=True
        )
        self.assertEqual(0, help_result.returncode)
        self.assertIn("round-trip", help_result.stdout)

    def test_probe_usernames_fit_the_protocol_limit(self):
        tokens = SCRIPT.read_text(encoding="utf-8").split()
        usernames = [
            tokens[index + 1]
            for index, token in enumerate(tokens[:-1])
            if token == "--username"
        ]
        self.assertTrue(usernames)
        for username in usernames:
            utf16_units = len(username.encode("utf-16-le")) // 2
            self.assertLessEqual(utf16_units, 16, username)

    def test_eula_gate_exits_2_before_any_work(self):
        with tempfile.TemporaryDirectory() as directory:
            result = self.run_gate({"BBB_SERVER_ROUND_TRIP_ARTIFACT_DIR": directory})
            self.assertEqual(2, result.returncode)
            self.assertIn("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA", result.stderr)
            self.assertEqual([], list(Path(directory).iterdir()))

    def test_missing_client_binary_exits_2(self):
        result = self.run_gate(
            {
                "BBB_OFFICIAL_CAPTURE_ACCEPT_EULA": "1",
                "BBB_SERVER_PLAYABLE_CLIENT_BIN": "/nonexistent/bbb-client",
                "BBB_SERVER_PLAYABLE_SERVER_BIN": "/nonexistent/bbb-server",
            }
        )
        self.assertEqual(2, result.returncode)
        self.assertIn("bbb-client is unavailable", result.stderr)

    def test_missing_server_binary_exits_2(self):
        with tempfile.TemporaryDirectory() as directory:
            fake_client = Path(directory) / "bbb-client"
            fake_client.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            fake_client.chmod(0o755)
            result = self.run_gate(
                {
                    "BBB_OFFICIAL_CAPTURE_ACCEPT_EULA": "1",
                    "BBB_SERVER_PLAYABLE_CLIENT_BIN": str(fake_client),
                    "BBB_SERVER_PLAYABLE_SERVER_BIN": "/nonexistent/bbb-server",
                }
            )
            self.assertEqual(2, result.returncode)
            self.assertIn("bbb-server is unavailable", result.stderr)

    def test_missing_admin_binary_exits_2(self):
        # The entity step drives BBB's `/tick freeze` and `/tick step` through
        # the management endpoint, so bbb-admin is as much a prerequisite of
        # this gate as the server it administers.
        with tempfile.TemporaryDirectory() as directory:
            stub = Path(directory) / "stub"
            stub.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            stub.chmod(0o755)
            result = self.run_gate(
                {
                    "BBB_OFFICIAL_CAPTURE_ACCEPT_EULA": "1",
                    "BBB_SERVER_PLAYABLE_CLIENT_BIN": str(stub),
                    "BBB_SERVER_PLAYABLE_SERVER_BIN": str(stub),
                    "BBB_SERVER_ROUND_TRIP_ADMIN_BIN": "/nonexistent/bbb-admin",
                }
            )
            self.assertEqual(2, result.returncode)
            self.assertIn("bbb-admin is unavailable", result.stderr)


class EntityStepTests(unittest.TestCase):
    """The entity step is wired into the gate, not merely available beside it.

    The helper can be perfect and prove nothing if the gate never calls it, or
    calls it on a save it then throws away. These read the script itself,
    because the run that would notice costs an official world capture.
    """

    def setUp(self):
        self.text = SCRIPT.read_text(encoding="utf-8")

    def test_bbb_writes_the_entity_and_the_official_server_reads_it_back(self):
        self.assertIn("scripts/server_round_trip_entity.py\" write", self.text)
        self.assertIn("scripts/server_round_trip_entity.py\" verify", self.text)

    def test_the_reload_is_compared_against_what_bbb_published(self):
        # Without this the check would pass on a file the official server never
        # opened, which is the one shape "the entity is still there" cannot see.
        self.assertIn('--published "$artifact_dir/entity-write.json"', self.text)
        self.assertIn('>"$artifact_dir/entity-write.json"', self.text)

    def test_the_official_resume_opens_the_world_the_entity_step_published(self):
        self.assertIn('cp -a "$entity_world" "$official_dir/world"', self.text)
        self.assertIn('--world "$official_dir/world"', self.text)

    def test_the_official_server_is_frozen_before_the_block_can_land(self):
        # A landed falling block is discarded, so an unfrozen reload would read
        # an empty entity storage for the one reason the step cannot tell from
        # failure.
        freeze = self.text.index("printf 'tick freeze\\nsave-all flush\\n' >&3")
        verify = self.text.index("server_round_trip_entity.py\" verify")
        unfreeze = self.text.index("printf 'tick unfreeze\\n' >&3")
        probe = self.text.index("--probe-server")
        self.assertLess(freeze, verify)
        self.assertLess(verify, unfreeze)
        self.assertLess(unfreeze, probe)

    def test_the_official_log_check_is_one_function_used_on_both_sides(self):
        self.assertEqual(1, self.text.count("official resume logged errors"))
        entity_step = self.text[: self.text.index('noise_helper="$repo_root')]
        self.assertEqual(2, entity_step.count("\nrequire_clean_official_log\n"))

    def test_the_log_check_ignores_only_probe_authored_refusal_exceptions(self):
        self.assertIn("Signed command mismatch between server and client ('list')", self.text)
        for username, packet in [
            ("refuse-config", "Client acknowledged config, but none was requested"),
            ("refuse-body", "serverbound/minecraft:chat"),
            ("refuse-nbt", "serverbound/minecraft:set_creative_mode_slot"),
        ]:
            self.assertIn(f"{username} lost connection: Internal Exception", self.text)
            self.assertIn(packet, self.text)
        self.assertIn("grep -E 'ERROR|Exception|Caused by'", self.text)
        self.assertNotIn("grep -v 'Exception'", self.text)


class PersistencePreflightTests(unittest.TestCase):
    def setUp(self):
        self.text = SCRIPT.read_text(encoding="utf-8")

    def test_authorized_persistence_tests_use_their_current_crate(self):
        self.assertIn(
            'test -p bbb-server-persistence -- --ignored', self.text
        )
        self.assertNotIn('test -p bbb-server-core -- --ignored', self.text)


class ProbeBudgetTests(unittest.TestCase):
    def setUp(self):
        self.text = SCRIPT.read_text(encoding="utf-8")

    def probe_command(self, username):
        username_index = self.text.index(f"--username {username}")
        start = self.text.rindex('"$repo_root/scripts/process" run', 0, username_index)
        end = self.text.index(">", username_index)
        return self.text[start:end]

    def test_all_three_probes_have_separate_client_and_process_deadlines(self):
        self.assertEqual(3, self.text.count("--probe-timeout-seconds"))
        self.assertEqual(3, self.text.count('"$repo_root/scripts/process" run --timeout'))

        expected = {
            "round-trip-probe": (
                "$probe_process_timeout_seconds",
                "$probe_timeout_seconds",
            ),
            "noise-bbb-probe": (
                "$noise_probe_process_timeout_seconds",
                "$noise_probe_timeout_seconds",
            ),
            "noise-java-probe": (
                "$noise_probe_process_timeout_seconds",
                "$noise_probe_timeout_seconds",
            ),
        }
        for username, (process_budget, client_budget) in expected.items():
            command = self.probe_command(username)
            self.assertIn(f'run --timeout "{process_budget}"', command)
            self.assertIn(f'--probe-timeout-seconds "{client_budget}"', command)

    def test_process_deadlines_leave_client_diagnostics_ten_seconds(self):
        self.assertIn(
            "probe_process_timeout_seconds=$((probe_timeout_seconds + 10))",
            self.text,
        )
        self.assertIn(
            "noise_probe_process_timeout_seconds=$((noise_probe_timeout_seconds + 10))",
            self.text,
        )
        self.assertNotIn('run --timeout "$probe_timeout_seconds"', self.text)
        self.assertNotIn('run --timeout "$noise_probe_timeout_seconds"', self.text)

    def test_noise_publish_and_resume_share_an_independent_long_budget(self):
        if os.name == "posix":
            help_text = subprocess.run(
                ["sh", str(SCRIPT), "--help"], capture_output=True, text=True, check=True
            ).stdout
            self.assertIn("BBB_SERVER_PROBE_TIMEOUT_SECONDS", help_text)
            self.assertIn("BBB_SERVER_NOISE_PROBE_TIMEOUT_SECONDS", help_text)
        self.assertIn(
            "probe_timeout_seconds=${BBB_SERVER_PROBE_TIMEOUT_SECONDS:-90}",
            self.text,
        )
        self.assertIn(
            "noise_probe_timeout_seconds=${BBB_SERVER_NOISE_PROBE_TIMEOUT_SECONDS:-600}",
            self.text,
        )
        self.assertIn(
            '--probe-timeout-seconds "$noise_probe_timeout_seconds"',
            self.probe_command("noise-bbb-probe"),
        )
        self.assertIn(
            '--probe-timeout-seconds "$noise_probe_timeout_seconds"',
            self.probe_command("noise-java-probe"),
        )


class NoiseResumeStepTests(unittest.TestCase):
    def setUp(self):
        self.text = SCRIPT.read_text(encoding="utf-8")

    def test_noise_handoff_keeps_official_world_settings_and_clears_chunks(self):
        self.assertIn("level-seed=0", self.text)
        self.assertNotIn("level-seed=bbb-noise-round-trip-26.1", self.text)
        self.assertIn("level-type=minecraft:normal", self.text)
        self.assertIn("generate-structures=true", self.text)
        self.assertIn('"$noise_helper" prepare', self.text)
        self.assertIn('--input-world "$noise_seed_dir/world"', self.text)
        self.assertIn('--output-world "$noise_bbb_client_world"', self.text)

    def test_noise_resume_follows_the_frozen_handoff_order(self):
        prepare = self.text.index('"$noise_helper" prepare')
        bbb_start = self.text.index('"$server_bin" --bind "127.0.0.1:$noise_bbb_port"')
        probe = self.text.index("--username noise-bbb-probe", bbb_start)
        playable_assert = self.text.index('server_playable_gate.py" assert', probe)
        inspect = self.text.index('"$noise_helper" inspect')
        official_resume = self.text.index(
            "resuming BBB Noise FULL chunks on the official 26.1 server"
        )
        force_load = self.text.index("forceload add", official_resume)
        force_load_wait = self.text.index("wait_marker 'Marked chunk'", force_load)
        conditional_marker = self.text.index("execute if block", official_resume)
        marker_wait = self.text.index(
            "wait_marker 'bbb-noise-full-marker-loaded'", conditional_marker
        )
        control_write = self.text.index("setblock %s %s %s minecraft:gold_block")
        control_wait = self.text.index("wait_marker 'Changed the block at'", control_write)
        official_probe = self.text.index(
            "--username noise-java-probe", control_wait
        )
        official_assert = self.text.index(
            'server_playable_gate.py" assert', official_probe
        )
        flush = self.text.index("printf 'save-all flush\\n'", control_write)
        verify = self.text.index('"$noise_helper" verify')
        success = self.text.index("official→BBB→official round-trip passed", verify)
        self.assertLess(prepare, bbb_start)
        self.assertLess(bbb_start, probe)
        self.assertLess(probe, playable_assert)
        self.assertLess(playable_assert, inspect)
        self.assertLess(inspect, official_resume)
        self.assertLess(official_resume, force_load)
        self.assertLess(force_load, force_load_wait)
        self.assertLess(force_load_wait, conditional_marker)
        self.assertLess(conditional_marker, marker_wait)
        self.assertLess(marker_wait, control_write)
        self.assertLess(control_write, control_wait)
        self.assertLess(control_wait, official_probe)
        self.assertLess(official_probe, official_assert)
        self.assertLess(official_assert, flush)
        self.assertLess(flush, verify)
        self.assertLess(verify, success)

    def test_noise_resume_opens_the_bbb_published_world(self):
        self.assertIn('cp -a "$noise_bbb_client_world" "$noise_official_dir/world"', self.text)
        self.assertIn('--world "$noise_official_dir/world"', self.text)

    def test_noise_verify_receives_the_published_receipt(self):
        self.assertIn('>"$noise_published_json"', self.text)
        self.assertIn('--published "$noise_published_json"', self.text)

    def test_noise_resume_checks_clean_logs(self):
        seed_start = self.text.index("creating official normal Overworld settings")
        resume_start = self.text.index("resuming BBB Noise FULL chunks on the official 26.1 server")
        verify = self.text.index('"$noise_helper" verify')
        self.assertIn("require_clean_official_log", self.text[seed_start:resume_start])
        self.assertIn("require_clean_official_log", self.text[resume_start:verify])

    def test_bbb_marker_must_load_before_the_control_write(self):
        force_load = self.text.index("forceload add")
        force_load_wait = self.text.index("wait_marker 'Marked chunk'", force_load)
        marker = self.text.index("execute if block")
        marker_wait = self.text.index("wait_marker 'bbb-noise-full-marker-loaded'", marker)
        control_write = self.text.index("setblock %s %s %s minecraft:gold_block", marker_wait)
        self.assertLess(force_load, force_load_wait)
        self.assertLess(force_load_wait, marker)
        self.assertLess(marker, marker_wait)
        self.assertLess(marker_wait, control_write)

    def test_official_noise_resume_is_served_to_a_real_probe(self):
        official_resume = self.text.index(
            "resuming BBB Noise FULL chunks on the official 26.1 server"
        )
        official_probe = self.text.index(
            "--username noise-java-probe", official_resume
        )
        official_assert = self.text.index(
            'server_playable_gate.py" assert', official_probe
        )
        flush = self.text.index("printf 'save-all flush\\n'", official_assert)
        self.assertIn(
            '--server "127.0.0.1:$noise_official_port"',
            self.text[official_resume:official_probe],
        )
        self.assertLess(official_probe, official_assert)
        self.assertLess(official_assert, flush)


if __name__ == "__main__":
    unittest.main()
