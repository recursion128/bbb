"""bbb is developed on Linux and macOS; the dev chain has to run on both.

Only one of those hosts gates this repository, so nothing here would notice a
GNU-only construct creeping into a script a macOS developer runs every day.
These checks are that notice: they read the scripts the dev loop and the full
gate execute, and reject the constructs whose BSD userland spelling differs.

The Linux-only *release* gates are deliberately out of scope — they are allowed
to be Linux-only, but they must say so instead of failing on a missing device.
"""

from pathlib import Path
import re
import subprocess
import tomllib
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"

# Scripts a macOS developer runs: the build/test loop, the full gate and its
# helpers, plus the generators kept next to them.
DEV_CHAIN_SHELL_SCRIPTS = (
    "bootstrap",
    "cargo-dev.sh",
    "docs-lint.sh",
    "gate-stamp-lib.sh",
    "install-hooks.sh",
    "python",
    "vanilla-source-tools.sh",
    "worker-worktree.sh",
    "generate-block-registry-catalog.sh",
    "generate-furnace-fuel-catalog.sh",
    "generate-item-registry-catalog.sh",
    "generate-sound-type-catalog.sh",
)

# Scripts that legitimately need Linux, each with the host guard it must carry
# so the failure names the platform rather than a missing kernel interface.
LINUX_ONLY_SCRIPTS = {
    "seat_device.py": "require_linux_host",
    "seat_inject.py": "require_linux_host",
    "gpu_duration_capture.py": 'sys.platform != "linux"',
    "platform-acceptance-gate.sh": 'host_os" != "Linux',
}

# Constructs whose GNU spelling is not what BSD/macOS userland accepts. Each
# entry is (regex, portable替代) and matches command position only, so prose in
# comments and `--help` text does not trip it.
GNU_ONLY_CONSTRUCTS = (
    (r"(?<![\w-])readlink\s+-f\b", "cd/pwd in a subshell"),
    (r"(?<![\w-])sha256sum\b", "a Python hashlib call"),
    (r"(?<![\w-])sha1sum\b", "a Python hashlib call"),
    (r"(?<![\w-])md5sum\b", "a Python hashlib call"),
    (r"(?<![\w-])stat\s+-c\b", "a Python os.stat call"),
    (r"(?<![\w-])date\s+-d\b", "a Python datetime call"),
    (r"(?<![\w-])date\s+--", "a Python datetime call"),
    (r"(?<![\w-])sed\s+-i\s", "a temporary file and mv"),
    (r"(?<![\w-])sed\s+-r\b", "sed -E"),
    (r"(?<![\w-])grep\s+-P\b", "grep -E"),
    (r"(?<![\w-])nproc\b", "getconf _NPROCESSORS_ONLN"),
    (r"(?<![\w-])xargs\s+-r\b", "a guard on the empty case"),
    (r"(?<![\w-])tac\b", "sed '1!G;h;$!d'"),
    (r"(?<![\w-])realpath\b", "cd/pwd in a subshell"),
    (r"(?<![\w-])seq\s", "a shell while loop"),
    (r"(?<![\w-])base64\s+-w\b", "tr -d '\\n'"),
    (r"(?<![\w-])cp\s+--", "short cp flags"),
    (r"(?<![\w-])ls\s+--", "short ls flags"),
    (r"-printf\b", "-exec printf"),
)

COMMENT_RE = re.compile(r"^\s*#")


def dev_chain_lines(path: Path):
    """Executable lines only — comments describe, they do not run."""
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if COMMENT_RE.match(line):
            continue
        yield number, line


class DevChainShellPortabilityTests(unittest.TestCase):
    def test_every_named_script_exists(self) -> None:
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            with self.subTest(script=name):
                self.assertTrue((SCRIPTS / name).is_file())

    def test_no_gnu_only_constructs(self) -> None:
        offenders = []
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            path = SCRIPTS / name
            for number, line in dev_chain_lines(path):
                for pattern, replacement in GNU_ONLY_CONSTRUCTS:
                    if re.search(pattern, line):
                        offenders.append(
                            f"{name}:{number}: {pattern} is GNU-only; use {replacement}"
                        )
        self.assertEqual(offenders, [])

    def test_shebangs_are_portable(self) -> None:
        """`/bin/sh` or an env-resolved bash; never a hardcoded /bin/bash path.

        macOS ships bash 3.2 at /bin/bash and modern bash under Homebrew, so a
        script that needs bash must find it through PATH.
        """
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            path = SCRIPTS / name
            first = path.read_text(encoding="utf-8").splitlines()[0]
            if not first.startswith("#!"):
                # gate-stamp-lib.sh is sourced, not executed.
                continue
            with self.subTest(script=name):
                self.assertIn(
                    first, ("#!/bin/sh", "#!/usr/bin/env bash"), f"{name}: {first}"
                )

    def test_bash_only_scripts_declare_bash(self) -> None:
        """A `/bin/sh` script may not use bashisms: macOS /bin/sh is not bash."""
        bashisms = (
            # `[[:space:]]` inside a regex is a POSIX class, not a bash test.
            re.compile(r"(?<![\w-])\[\[(?!:)"),
            re.compile(r"(?<![\w-])local\s"),
            re.compile(r"\$\{BASH_SOURCE"),
            re.compile(r"(?<![\w-])declare\s"),
            re.compile(r"=\(\s*$"),
        )
        offenders = []
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            path = SCRIPTS / name
            text = path.read_text(encoding="utf-8")
            if not text.startswith("#!/bin/sh"):
                continue
            for number, line in dev_chain_lines(path):
                for bashism in bashisms:
                    if bashism.search(line):
                        offenders.append(f"{name}:{number}: {line.strip()}")
        self.assertEqual(offenders, [])


class LinuxOnlyScriptTests(unittest.TestCase):
    def test_linux_only_scripts_name_their_host_requirement(self) -> None:
        for name, guard in LINUX_ONLY_SCRIPTS.items():
            with self.subTest(script=name):
                text = (SCRIPTS / name).read_text(encoding="utf-8")
                self.assertIn(guard, text)

    def test_the_guards_say_which_host_is_running(self) -> None:
        """The message has to be actionable, not just a refusal."""
        for name in LINUX_ONLY_SCRIPTS:
            with self.subTest(script=name):
                text = (SCRIPTS / name).read_text(encoding="utf-8")
                self.assertRegex(text, r"this host is")


class SupportedPlatformTests(unittest.TestCase):
    """bbb supports macOS and Linux. Windows is a declared non-goal.

    The removal has to stay removed, and nothing here compiles a Windows
    target, so a Windows arm that grew back would be invisible: no host would
    build it and no test would reach it. These checks are that visibility.
    """

    CRATES = REPO_ROOT / "crates"
    # The one legal mention: the guard that refuses the target outright.
    BUILD_REFUSAL = 'compile_error!(\n    "bbb supports macOS and Linux'

    def rust_sources(self):
        for path in sorted(self.CRATES.rglob("*.rs")):
            yield path, path.read_text(encoding="utf-8")

    def test_the_build_refuses_a_windows_target(self) -> None:
        main = (self.CRATES / "bbb-native" / "src" / "main.rs").read_text(encoding="utf-8")
        self.assertIn('#[cfg(target_os = "windows")]', main)
        self.assertIn(self.BUILD_REFUSAL, main)

    def test_no_crate_carries_windows_conditional_code(self) -> None:
        offenders = []
        for path, text in self.rust_sources():
            if self.BUILD_REFUSAL in text:
                # main.rs's refusal is the guard itself.
                text = text.replace('#[cfg(target_os = "windows")]', "", 1)
            for marker in ('target_os = "windows"', "target_family = \"windows\""):
                if marker in text:
                    offenders.append(f"{path.relative_to(REPO_ROOT)}: {marker}")
            if "cfg!(windows)" in text or "#[cfg(windows)]" in text:
                offenders.append(f"{path.relative_to(REPO_ROOT)}: cfg(windows)")
        self.assertEqual(offenders, [])

    def test_no_crate_reaches_a_windows_only_api(self) -> None:
        windows_only = (
            "WindowExtWindows",
            "winit::platform::windows",
            "rundll32",
            "FileProtocolHandler",
            'env("APPDATA")',
            'var_os("APPDATA")',
        )
        offenders = []
        for path, text in self.rust_sources():
            for marker in windows_only:
                if marker in text:
                    offenders.append(f"{path.relative_to(REPO_ROOT)}: {marker}")
        self.assertEqual(offenders, [])

    def test_the_windows_options_default_is_a_scope_constant(self) -> None:
        """Vanilla's `syncChunkWrites` default reads `Util.getPlatform()`.

        The catalog keeps that derivation, but with no Windows target the host
        side is false by construction; it must come from the platform owner
        rather than from a `cfg!` nobody can evaluate.
        """
        platform = (self.CRATES / "bbb-native" / "src" / "platform.rs").read_text(
            encoding="utf-8"
        )
        self.assertIn("pub(crate) const HOST_IS_WINDOWS: bool = false;", platform)
        for name in ("main.rs", "startup.rs"):
            source = (self.CRATES / "bbb-native" / "src" / name).read_text(encoding="utf-8")
            with self.subTest(source=name):
                self.assertIn(
                    "platform_is_windows: crate::platform::HOST_IS_WINDOWS,", source
                )


class DevChainToolTests(unittest.TestCase):
    """The external tools the dev chain assumes, kept in step with the Wiki.

    A macOS developer installs these by hand, so the list has to stay true.
    """

    EXPECTED_TOOLS = ("cargo", "git", "java", "rg", "rustup", "uv")

    def test_the_documented_tool_list_matches_this_one(self) -> None:
        page = (
            REPO_ROOT / "docs" / "wiki" / "development" / "macos-and-linux.md"
        ).read_text(encoding="utf-8")
        for tool in self.EXPECTED_TOOLS:
            with self.subTest(tool=tool):
                self.assertIn(f"`{tool}`", page)

    def test_the_dev_chain_uses_no_undocumented_external_tool(self) -> None:
        undocumented = {
            "wget": r"(?<![\w-])wget\s",
            "curl": r"(?<![\w-])curl\s",
            "jq": r"(?<![\w-])jq\s",
            "docker": r"(?<![\w-])docker\s",
        }
        offenders = []
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            path = SCRIPTS / name
            for number, line in dev_chain_lines(path):
                for tool, pattern in undocumented.items():
                    if re.search(pattern, line):
                        offenders.append(f"{name}:{number}: uses {tool}")
        self.assertEqual(offenders, [])


class PythonToolchainContractTests(unittest.TestCase):
    PINNED_PYTHON = "3.12.13"
    PINNED_UV = "0.11.16"

    def test_root_project_pins_the_interpreter_uv_and_dependencies(self) -> None:
        self.assertEqual(
            (REPO_ROOT / ".python-version").read_text(encoding="utf-8").strip(),
            self.PINNED_PYTHON,
        )
        project = tomllib.loads((REPO_ROOT / "pyproject.toml").read_text(encoding="utf-8"))
        self.assertEqual(project["project"]["requires-python"], f"=={self.PINNED_PYTHON}")
        self.assertEqual(project["tool"]["uv"]["required-version"], f"=={self.PINNED_UV}")
        self.assertEqual(project["tool"]["uv"]["python-preference"], "only-managed")
        self.assertEqual(project["tool"]["uv"]["python-downloads"], "manual")
        lock = (REPO_ROOT / "uv.lock").read_text(encoding="utf-8")
        self.assertIn(f'requires-python = "=={self.PINNED_PYTHON}"', lock)
        self.assertFalse((SCRIPTS / "vanilla-source-tools" / "uv.lock").exists())

    def test_daily_runner_is_offline_locked_exact_and_never_downloads_python(self) -> None:
        runner = (SCRIPTS / "python").read_text(encoding="utf-8")
        for flag in ("--offline", "--locked", "--exact", "--no-python-downloads"):
            with self.subTest(flag=flag):
                self.assertIn(flag, runner)
        self.assertIn('if [ ! -x "$repo_root/.venv/bin/python" ]', runner)

    def test_stable_python_entrypoints_share_the_runner(self) -> None:
        for name in ("dev", "input", "process", "vanilla", "vanilla-source-tools.sh"):
            source = (SCRIPTS / name).read_text(encoding="utf-8")
            with self.subTest(entrypoint=name):
                self.assertIn("scripts/python", source)

    def test_bootstrap_owns_every_networked_toolchain_prepare(self) -> None:
        bootstrap = (SCRIPTS / "bootstrap").read_text(encoding="utf-8")
        self.assertIn(f"required_uv={self.PINNED_UV}", bootstrap)
        for command in (
            'python install $uv_network',
            'sync --locked $uv_network',
            'cargo fetch --manifest-path "$repo_root/Cargo.toml" --locked',
            'rustup target add "$cross_target"',
            '"$repo_root/scripts/vanilla" prepare 26.1',
            '"$repo_root/scripts/vanilla" verify 26.1',
        ):
            with self.subTest(command=command):
                self.assertIn(command, bootstrap)

    def test_the_prepared_runner_uses_the_exact_managed_python(self) -> None:
        result = subprocess.run(
            [
                str(SCRIPTS / "python"),
                "python",
                "-c",
                "import platform; print(platform.python_version())",
            ],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.stdout.strip(), self.PINNED_PYTHON)

    def test_bootstrap_help_needs_no_prepared_environment(self) -> None:
        result = subprocess.run(
            [str(SCRIPTS / "bootstrap"), "--help"],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn("--offline", result.stdout)
        self.assertIn("--skip-vanilla", result.stdout)


if __name__ == "__main__":
    unittest.main()
