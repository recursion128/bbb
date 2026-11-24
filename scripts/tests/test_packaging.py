"""The supervisor and gateway files have to match the binaries this repository builds.

A unit file or reverse-proxy config is only discovered to be wrong when a host
tries to boot it, which is the worst place to find out that a flag was renamed
or cross-origin isolation headers were omitted. These tests read the CLI option
structs directly out of `crates/bbb-server/src/options.rs` and
`crates/bbb-web-host/src/main.rs` — the authoritative declarations of both
command lines — and refuse packaged configs that drift from them.
"""

from __future__ import annotations

import configparser
import os
import plistlib
import re
import subprocess
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PACKAGING = REPO_ROOT / "packaging"
CI_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"
CLIENT_SOURCE = REPO_ROOT / "crates" / "bbb-client" / "src" / "startup.rs"
OPTIONS_SOURCE = REPO_ROOT / "crates" / "bbb-server" / "src" / "options.rs"
WEB_HOST_SOURCE = REPO_ROOT / "crates" / "bbb-web-host" / "src" / "main.rs"

#: `#[arg(long)]` derives a flag from the field name below it, with underscores
#: turned into dashes; `#[arg(long = "...")]` overrides the derived flag name.
EXPLICIT_LONG_RE = re.compile(r'\blong\s*=\s*"([^"]+)"')
HELP_LONG_FLAG_RE = re.compile(r"(?<!\w)(--[a-z0-9][a-z0-9-]*)")
HELP_OPTION_HEADER_RE = re.compile(
    r"^\s{2,6}(?:-[A-Za-z],\s+)?(--[a-z0-9][a-z0-9-]*)\b", re.MULTILINE
)
CLAP_FIELD_NAME_RE = re.compile(r"\s*(?:pub(?:\([^)]*\))?\s+)?([a-z_0-9]+)\s*:")

#: Service release targets (`bbb-server`, `bbb-admin`, `bbb-web-host`) and
#: desktop client release targets (`bbb-client`).
SERVICE_RELEASE_TARGETS = ("x86_64-unknown-linux-gnu", "aarch64-apple-darwin")
CLIENT_RELEASE_TARGETS = ("x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc")
RELEASE_TARGETS = tuple(dict.fromkeys((*SERVICE_RELEASE_TARGETS, *CLIENT_RELEASE_TARGETS)))


def _parse_clap_block(block: str) -> tuple[set[str], set[str]]:
    all_flags: set[str] = set()
    visible_flags: set[str] = set()

    pos = 0
    while True:
        idx = block.find("#[arg(", pos)
        if idx == -1:
            break
        cursor = idx + len("#[arg(")
        depth = 1
        in_string = False
        escaped = False
        while cursor < len(block) and depth > 0:
            ch = block[cursor]
            if in_string:
                if escaped:
                    escaped = False
                elif ch == "\\":
                    escaped = True
                elif ch == '"':
                    in_string = False
            else:
                if ch == '"':
                    in_string = True
                elif ch == "(":
                    depth += 1
                elif ch == ")":
                    depth -= 1
            cursor += 1
        attr_args = block[idx + len("#[arg(") : cursor - 1]
        pos = cursor
        if cursor >= len(block) or block[cursor] != "]":
            continue
        match = CLAP_FIELD_NAME_RE.match(block, cursor + 1)
        if match is None:
            continue
        field_name = match.group(1)
        if re.search(r"(?:^|,)\s*skip(?:\s*=|\s*$)", attr_args):
            continue
        explicit = EXPLICIT_LONG_RE.search(attr_args)
        if explicit is not None:
            flag = f"--{explicit.group(1)}"
        elif re.search(r"\blong\b", attr_args):
            flag = "--" + field_name.replace("_", "-")
        else:
            continue
        all_flags.add(flag)
        if not re.search(r"\bhide\s*=\s*true\b", attr_args):
            visible_flags.add(flag)

    return all_flags, visible_flags


def parse_clap_struct_flags(source: Path, struct_header: str) -> set[str]:
    body = source.read_text(encoding="utf-8")
    start = body.index(struct_header)
    end = body.index("\n}", start)
    all_flags, _visible = _parse_clap_block(body[start:end])
    return all_flags


def parse_client_startup_flags(source: Path = CLIENT_SOURCE) -> tuple[set[str], set[str]]:
    body = source.read_text(encoding="utf-8")
    start = body.index("define_startup_args! {")
    end = body.index("\n}", start)
    return _parse_clap_block(body[start:end])


def client_flags() -> set[str]:
    all_flags, _visible = parse_client_startup_flags()
    return all_flags


def server_flags() -> set[str]:
    return parse_clap_struct_flags(OPTIONS_SOURCE, "pub struct ServerOptions {")


def web_host_flags() -> set[str]:
    return parse_clap_struct_flags(WEB_HOST_SOURCE, "struct Cli {")


def systemd_exec_start(unit_name: str = "bbb-server.service") -> list[str]:
    body = (PACKAGING / unit_name).read_text(encoding="utf-8")
    match = re.search(r"^ExecStart=(.*?)(?=\n[A-Z#\[]|\n\n)", body, re.MULTILINE | re.DOTALL)
    assert match is not None, f"{unit_name} declares no ExecStart"
    return match.group(1).replace("\\\n", " ").split()


def launchd_arguments(plist_name: str = "com.bbb.server.plist") -> list[str]:
    with (PACKAGING / plist_name).open("rb") as handle:
        return plistlib.load(handle)["ProgramArguments"]


class PackagingTests(unittest.TestCase):
    def test_supervisors_pass_only_options_the_server_declares(self):
        flags = server_flags()
        self.assertIn("--bind", flags)
        self.assertIn("--max-players", flags)
        self.assertIn("--mod", flags)
        self.assertNotIn("--mods", flags)
        for name, arguments in (
            ("systemd", systemd_exec_start()),
            ("launchd", launchd_arguments()),
        ):
            with self.subTest(supervisor=name):
                passed = [word for word in arguments[1:] if word.startswith("--")]
                self.assertTrue(passed, "the packaged command line passes no options")
                unknown = sorted(set(passed) - flags)
                self.assertEqual(unknown, [], f"{name} passes options the server has no flag for")

    def test_both_supervisors_run_the_same_server_with_the_same_shape(self):
        systemd = systemd_exec_start()
        launchd = launchd_arguments()
        self.assertTrue(systemd[0].endswith("/bbb-server"))
        self.assertEqual(Path(systemd[0]).name, Path(launchd[0]).name)
        # The paths differ by platform convention; which options are passed must
        # not, or the two hosts would be running differently configured servers.
        self.assertEqual(
            [word for word in systemd[1:] if word.startswith("--")],
            [word for word in launchd[1:] if word.startswith("--")],
        )

    def test_the_management_endpoint_stays_on_loopback(self):
        for name, arguments in (
            ("systemd", systemd_exec_start()),
            ("launchd", launchd_arguments()),
        ):
            with self.subTest(supervisor=name):
                bind = arguments[arguments.index("--mcp-bind") + 1]
                self.assertTrue(
                    bind.startswith("127.0.0.1:"),
                    f"{name} publishes the management endpoint at {bind}",
                )

    def test_both_supervisors_stop_with_the_signal_the_server_saves_on(self):
        unit = (PACKAGING / "bbb-server.service").read_text(encoding="utf-8")
        self.assertIn("KillSignal=SIGTERM", unit)
        # A stop budget below the server's own shutdown budget would kill it
        # mid-save, so it has to be the larger of the two.
        stop_seconds = int(re.search(r"TimeoutStopSec=(\d+)", unit).group(1))
        default = int(
            re.search(
                r"default_value_t = (\d+).*?\n    pub shutdown_timeout_seconds",
                OPTIONS_SOURCE.read_text(encoding="utf-8"),
                re.DOTALL,
            ).group(1)
        )
        self.assertGreater(stop_seconds, default)
        with (PACKAGING / "com.bbb.server.plist").open("rb") as handle:
            self.assertGreater(plistlib.load(handle)["ExitTimeOut"], default)

    def test_web_host_flag_parser_respects_explicit_clap_overrides(self):
        flags = web_host_flags()
        self.assertIn("--bind", flags)
        self.assertIn("--server", flags)
        self.assertIn("--allow-server", flags)
        self.assertNotIn("--allow-servers", flags)
        self.assertIn("--vanilla-root", flags)
        self.assertIn("--wasm-dir", flags)
        self.assertIn("--cache-dir", flags)
        self.assertIn("--mcp-bind", flags)
        self.assertIn("--control", flags)

        # When a compiled bbb-web-host binary is already present in CARGO_TARGET_DIR,
        # cross-check `bbb-web-host --help` against the parsed struct flags as well.
        target_dir = os.environ.get("CARGO_TARGET_DIR")
        if target_dir:
            exe_name = "bbb-web-host.exe" if os.name == "nt" else "bbb-web-host"
            source_mtime = WEB_HOST_SOURCE.stat().st_mtime
            for profile in ("debug", "release"):
                candidate = Path(target_dir) / profile / exe_name
                if candidate.is_file() and candidate.stat().st_mtime >= source_mtime:
                    proc = subprocess.run(
                        [str(candidate), "--help"],
                        check=True,
                        text=True,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.STDOUT,
                    )
                    help_flags = set(HELP_LONG_FLAG_RE.findall(proc.stdout)) - {
                        "--help",
                        "--version",
                    }
                    self.assertEqual(flags, help_flags)
                    break

    def test_web_host_supervisors_pass_only_valid_flags_and_match_shape(self):
        flags = web_host_flags()
        systemd = systemd_exec_start("bbb-web-host.service")
        launchd = launchd_arguments("com.bbb.web-host.plist")
        self.assertTrue(systemd[0].endswith("/bbb-web-host"))
        self.assertEqual(Path(systemd[0]).name, Path(launchd[0]).name)

        for name, arguments in (("systemd", systemd), ("launchd", launchd)):
            with self.subTest(supervisor=name):
                passed = [word for word in arguments[1:] if word.startswith("--")]
                self.assertTrue(passed, f"{name} web-host command line passes no options")
                unknown = sorted(set(passed) - flags)
                self.assertEqual(
                    unknown, [], f"{name} passes options bbb-web-host has no flag for"
                )
                bind = arguments[arguments.index("--bind") + 1]
                self.assertTrue(
                    bind.startswith("127.0.0.1:"),
                    f"{name} binds bbb-web-host outside loopback ({bind}) instead of behind a TLS proxy",
                )
                for local_flag in ("--mcp-bind", "--control"):
                    if local_flag in arguments:
                        addr = arguments[arguments.index(local_flag) + 1]
                        self.assertTrue(
                            addr.startswith("127.0.0.1:"),
                            f"{name} exposes {local_flag} on non-loopback {addr}",
                        )

        self.assertEqual(
            [word for word in systemd[1:] if word.startswith("--")],
            [word for word in launchd[1:] if word.startswith("--")],
        )

    def test_reverse_proxy_configs_enforce_coop_coep_ws_and_block_control_ws(self):
        caddy = (PACKAGING / "Caddyfile").read_text(encoding="utf-8")
        nginx = (PACKAGING / "nginx-bbb-web.conf").read_text(encoding="utf-8")
        for name, text in (("Caddyfile", caddy), ("nginx-bbb-web.conf", nginx)):
            with self.subTest(proxy=name):
                self.assertRegex(
                    text, r'Cross-Origin-Opener-Policy\s+"same-origin"'
                )
                self.assertRegex(
                    text, r'Cross-Origin-Embedder-Policy\s+"require-corp"'
                )
                self.assertRegex(
                    text, r'Cross-Origin-Resource-Policy\s+"same-origin"'
                )
                self.assertIn("127.0.0.1:8080", text)
                self.assertIn("/control-ws", text)
                self.assertIn("403", text)
        self.assertIn("location = /ws", nginx)
        self.assertIn("Upgrade $http_upgrade", nginx)

    def test_client_flag_parser_matches_startup_args_and_help_when_built(self):
        all_flags, visible_flags = parse_client_startup_flags()
        for expected in (
            "--game-dir",
            "--vanilla-root",
            "--server",
            "--quick-play-multiplayer",
            "--status-server",
            "--connect-server",
            "--fullscreen",
            "--width",
            "--height",
            "--username",
            "--build-info",
            "--probe-server",
        ):
            self.assertIn(expected, all_flags)
        for skipped in (
            "--stored-resource-packs",
            "--stored-incompatible-resource-packs",
            "--client-key-bindings",
            "--client-audio-volumes",
        ):
            self.assertNotIn(skipped, all_flags)
        self.assertNotIn("--probe-server", visible_flags)

        target_dir = os.environ.get("CARGO_TARGET_DIR")
        if target_dir:
            exe_name = "bbb-client.exe" if os.name == "nt" else "bbb-client"
            source_mtime = CLIENT_SOURCE.stat().st_mtime
            for profile in ("debug", "release"):
                candidate = Path(target_dir) / profile / exe_name
                if candidate.is_file() and candidate.stat().st_mtime >= source_mtime:
                    proc = subprocess.run(
                        [str(candidate), "--help"],
                        check=True,
                        text=True,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.STDOUT,
                    )
                    help_flags = set(HELP_OPTION_HEADER_RE.findall(proc.stdout)) - {
                        "--help",
                        "--version",
                    }
                    self.assertEqual(visible_flags, help_flags)
                    break

    def test_client_launchers_and_desktop_entry_pass_only_valid_flags(self):
        flags = client_flags()
        expected_wrapper_flags = {
            "--game-dir",
            "--vanilla-root",
            "--server",
            "--quick-play-multiplayer",
            "--status-server",
            "--fullscreen",
        }

        desktop = configparser.ConfigParser(interpolation=None)
        desktop.optionxform = str
        desktop.read(PACKAGING / "bbb-client.desktop", encoding="utf-8")
        entry = desktop["Desktop Entry"]
        self.assertEqual(entry["Type"], "Application")
        self.assertEqual(entry["Terminal"], "false")
        self.assertTrue(entry["TryExec"].endswith("/bbb-client"))
        self.assertTrue(entry["Exec"].endswith("/bbb-client-launch"))
        self.assertIn("Fullscreen", entry["Actions"].split(";"))
        fullscreen_exec = desktop["Desktop Action Fullscreen"]["Exec"].split()
        self.assertIn("--fullscreen", fullscreen_exec)
        for section_name in ("Desktop Entry", "Desktop Action Fullscreen"):
            exec_words = desktop[section_name]["Exec"].split()
            passed = [w for w in exec_words[1:] if w.startswith("--")]
            self.assertEqual(sorted(set(passed) - flags), [], section_name)

        sh_text = (PACKAGING / "bbb-client-launch.sh").read_text(encoding="utf-8")
        ps1_text = (PACKAGING / "bbb-client-launch.ps1").read_text(encoding="utf-8")
        cmd_text = (PACKAGING / "bbb-client-launch.cmd").read_text(encoding="utf-8")
        self.assertTrue(sh_text.startswith("#!/bin/sh\n"))
        self.assertIn("bbb-client", sh_text)
        self.assertIn("bbb-client.exe", ps1_text)
        self.assertIn("bbb-client-launch.ps1", cmd_text)
        self.assertIn("-ExecutionPolicy Bypass", cmd_text)

        for name, text in (
            ("bbb-client-launch.sh", sh_text),
            ("bbb-client-launch.ps1", ps1_text),
        ):
            with self.subTest(launcher=name):
                passed = set(HELP_LONG_FLAG_RE.findall(text)) - {"--help", "--version"}
                self.assertEqual(
                    sorted(passed - flags),
                    [],
                    f"{name} passes options bbb-client does not declare",
                )
                self.assertEqual(
                    passed,
                    expected_wrapper_flags,
                    f"{name} diverged from normalized client launcher flag contract",
                )

    def test_the_recipe_names_both_supported_targets_and_external_target_dir(self):
        recipe = (PACKAGING / "README.md").read_text(encoding="utf-8")
        for target in RELEASE_TARGETS:
            self.assertIn(target, recipe)
            self.assertNotIn(
                f"target/{target}/release/",
                recipe,
                "packaging recipe must not assume repo-local target/ directory",
            )
            if target in SERVICE_RELEASE_TARGETS:
                self.assertIn(f"/tmp/bbb-target-main/{target}/release/", recipe)
            else:
                self.assertIn(rf"%TEMP%\bbb-target-main\{target}\release\bbb-client.exe", recipe)
        self.assertIn("CARGO_TARGET_DIR=/tmp/bbb-target-main", recipe)
        self.assertIn("--locked --release", recipe)
        for asset in (
            "bbb-client.desktop",
            "bbb-client-launch.sh",
            "bbb-client-launch.ps1",
            "bbb-client-launch.cmd",
            "bbb-web-host.service",
            "com.bbb.web-host.plist",
            "Caddyfile",
            "nginx-bbb-web.conf",
        ):
            self.assertIn(asset, recipe)
        for flag in ("--game-dir", "--vanilla-root", "--server", "--fullscreen"):
            self.assertIn(flag, recipe)
        table = re.search(
            r"## Supported release targets\n\n(\|.*?)\n\n", recipe, re.DOTALL
        )
        assert table is not None, "the recipe declares no target table"
        self.assertEqual(
            sorted(re.findall(r"`([a-z0-9_]+-[a-z0-9_]+-[a-z0-9_]+(?:-[a-z0-9_]+)?)`", table.group(1))),
            sorted(RELEASE_TARGETS),
        )

    def test_ci_workflow_defines_release_packaging_for_client_and_server(self):
        ci = CI_WORKFLOW.read_text(encoding="utf-8")
        for token in (
            'tags: ["v*"]',
            "build_release_artifacts:",
            "actions/upload-artifact@v4",
            "x86_64-unknown-linux-gnu",
            "x86_64-pc-windows-msvc",
            "aarch64-apple-darwin",
            "-p bbb-client",
            "-p bbb-server -p bbb-admin -p bbb-web-host",
            "build-web --release",
            "packaging/bbb-client-launch.sh",
            "packaging/bbb-client.desktop",
            r"packaging\bbb-client-launch.ps1",
            r"packaging\bbb-client-launch.cmd",
        ):
            self.assertIn(token, ci)


if __name__ == "__main__":
    unittest.main()
