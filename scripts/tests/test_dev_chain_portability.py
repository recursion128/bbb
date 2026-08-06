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
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"

# Scripts a macOS developer runs: the build/test loop, the full gate and its
# helpers, plus the generators kept next to them.
DEV_CHAIN_SHELL_SCRIPTS = (
    "cargo-dev.sh",
    "docs-lint.sh",
    "gate-stamp-lib.sh",
    "install-hooks.sh",
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


class DevChainToolTests(unittest.TestCase):
    """The external tools the dev chain assumes, kept in step with the Wiki.

    A macOS developer installs these by hand, so the list has to stay true.
    """

    EXPECTED_TOOLS = ("cargo", "git", "python3", "rg", "uv")

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


if __name__ == "__main__":
    unittest.main()
