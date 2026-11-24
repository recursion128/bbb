from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_vanilla_anchors", ROOT / "scripts" / "check_vanilla_anchors.py"
)
assert SPEC and SPEC.loader
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


def _git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True, text=True)


def _make_prepared_vanilla(root: Path) -> Path:
    vanilla = root / "vanilla"
    sources = vanilla / "sources" / "26.1"
    assets = sources / "assets" / "minecraft"
    downloads = vanilla / "downloads" / "26.1"
    state = vanilla / "state" / "26.1.json"
    assets.mkdir(parents=True)
    downloads.mkdir(parents=True)
    state.parent.mkdir(parents=True)
    state.write_text(json.dumps({"status": "complete"}), encoding="utf-8")
    (sources / "MinecraftServer.java").write_text(
        "package net.minecraft.server;\npublic class MinecraftServer {\n    void tick() {}\n}\n",
        encoding="utf-8",
    )
    return vanilla


class AnchorPatternTests(unittest.TestCase):
    def test_it_matches_both_spellings_and_nothing_else(self) -> None:
        found = [
            (match.group(1), match.group(2), match.group(3))
            for match in CHECK.ANCHOR.finditer(
                "see MinecraftServer.java:975-976 and DedicatedServer.java:285, "
                "unlike world.rs:1234 or 26.1 or Foo.json:9"
            )
        ]
        self.assertEqual(
            found,
            [
                ("MinecraftServer.java", "975", "976"),
                ("DedicatedServer.java", "285", None),
            ],
        )

    def test_a_dollar_sign_inner_class_file_is_still_a_file(self) -> None:
        match = CHECK.ANCHOR.search("BlockBehaviour$BlockStateBase.java:12")
        assert match is not None
        self.assertEqual(match.group(1), "BlockBehaviour$BlockStateBase.java")


class EdgeReportTests(unittest.TestCase):
    def test_an_annotation_edge_is_a_problem_on_either_end(self) -> None:
        self.assertEqual(
            CHECK.problems_with("   @Override", "   }"),
            ["first line is @Override"],
        )
        self.assertEqual(
            CHECK.problems_with("   void f() {", "@Override"),
            ["last line is @Override"],
        )

    def test_a_blank_edge_is_a_problem_on_either_end(self) -> None:
        self.assertEqual(
            CHECK.problems_with("", "   }"),
            ["first line is blank"],
        )
        self.assertEqual(
            CHECK.problems_with("   void f() {", "   "),
            ["last line is blank"],
        )

    def test_an_ordinary_range_has_nothing_to_report(self) -> None:
        self.assertEqual(CHECK.problems_with("   void f() {", "   }"), [])


class ResolutionTests(unittest.TestCase):
    def test_the_candidate_that_holds_the_range_is_the_one_used(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            short = root / "short" / "Same.java"
            long = root / "long" / "Same.java"
            short.parent.mkdir()
            long.parent.mkdir()
            short.write_text("one\ntwo\n", encoding="utf-8")
            long.write_text("".join(f"line {n}\n" for n in range(1, 21)), encoding="utf-8")
            index = {"Same.java": [short, long]}
            path, lines = CHECK.resolve(index, "Same.java", 10, 12)
            self.assertEqual(path, long)
            assert lines is not None
            self.assertEqual(lines[9], "line 10")

            missing, _ = CHECK.resolve(index, "Same.java", 100, 101)
            self.assertIsNone(missing)

    def test_source_roots_never_probes_legacy_home_mc_code(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            legacy = home / "Work" / "mc-code" / "sources" / "26.1"
            legacy.mkdir(parents=True)
            (legacy / "Foo.java").write_text("class Foo {}\n", encoding="utf-8")
            env = {
                "HOME": str(home),
                "USERPROFILE": str(home),
                "LOCALAPPDATA": str(home / "empty-local"),
                "XDG_CACHE_HOME": str(home / "empty-xdg"),
            }
            with patch.object(Path, "home", return_value=home):
                self.assertEqual(CHECK.source_roots(ROOT, environ=env), [])

    def test_skips_when_vanilla_absent_unless_required(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            empty = Path(directory) / "missing-vanilla"
            env = {
                k: v
                for k, v in os.environ.items()
                if k not in ("BBB_VANILLA_ROOT", "BBB_REQUIRE_VANILLA")
            }
            env["BBB_VANILLA_ROOT"] = str(empty)
            with patch.dict(os.environ, env, clear=True):
                buf = io.StringIO()
                with contextlib.redirect_stdout(buf):
                    code = CHECK.main(["--repo-root", str(ROOT)])
                self.assertEqual(code, 0)
                self.assertIn("skipped", buf.getvalue())

            env["BBB_REQUIRE_VANILLA"] = "1"
            with patch.dict(os.environ, env, clear=True):
                buf = io.StringIO()
                with contextlib.redirect_stdout(buf):
                    code = CHECK.main(["--repo-root", str(ROOT)])
                self.assertEqual(code, 2)
                self.assertIn("no Vanilla source tree", buf.getvalue())

    def test_committed_branch_diffs_against_main_merge_base_when_worktree_clean(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            vanilla = _make_prepared_vanilla(root)
            repo = root / "repo"
            repo.mkdir()
            _git(repo, "init", "-q", "-b", "main")
            _git(repo, "config", "user.name", "Anchor Test")
            _git(repo, "config", "user.email", "anchor@example.invalid")
            (repo / "src.rs").write_text("fn a() {}\n", encoding="utf-8")
            _git(repo, "add", "src.rs")
            _git(repo, "commit", "-qm", "base")

            _git(repo, "checkout", "-qb", "worker/test")
            (repo / "src.rs").write_text(
                "// Valid: MinecraftServer.java:2-4\nfn a() {}\n", encoding="utf-8"
            )
            _git(repo, "add", "src.rs")
            _git(repo, "commit", "-qm", "commit 1")
            (repo / "other.rs").write_text(
                "// Bad: MissingClass.java:10\n", encoding="utf-8"
            )
            _git(repo, "add", "other.rs")
            _git(repo, "commit", "-qm", "commit 2")

            env = dict(os.environ)
            env["BBB_VANILLA_ROOT"] = str(vanilla)
            with patch.dict(os.environ, env, clear=True):
                buf = io.StringIO()
                with contextlib.redirect_stdout(buf):
                    code = CHECK.main(["--repo-root", str(repo)])
                output = buf.getvalue()
                self.assertEqual(code, 1, output)
                self.assertIn("MinecraftServer.java:2-4", output)
                self.assertIn("MISSING FILE            MissingClass.java:10-10", output)

    def test_package_hint_disambiguates_duplicate_basenames(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / "net/minecraft/world/entity/Selector.java"
            second = root / "net/minecraft/commands/Selector.java"
            first.parent.mkdir(parents=True)
            second.parent.mkdir(parents=True)
            first.write_text("\n".join(f"world {n}" for n in range(1, 31)) + "\n", encoding="utf-8")
            second.write_text("\n".join(f"cmd {n}" for n in range(1, 15)) + "\n", encoding="utf-8")
            index = {"Selector.java": [first, second]}
            path, _ = CHECK.resolve(
                index, "Selector.java", 5, 10, package_hint="commands"
            )
            self.assertEqual(path, second)
            oor_path, _ = CHECK.resolve(
                index, "Selector.java", 20, 25, package_hint="commands"
            )
            self.assertIsNone(oor_path)

    def test_all_and_empty_diff_fallback_and_library_jar_skip(self) -> None:
        import zipfile

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            vanilla = _make_prepared_vanilla(root)
            lib_dir = vanilla / "downloads" / "26.1" / "libraries" / "com" / "mojang"
            lib_dir.mkdir(parents=True)
            with zipfile.ZipFile(lib_dir / "brigadier.jar", "w") as archive:
                archive.writestr("com/mojang/brigadier/StringReader.class", b"")

            repo = root / "repo"
            repo.mkdir()
            _git(repo, "init", "-q", "-b", "main")
            _git(repo, "config", "user.name", "Anchor Test")
            _git(repo, "config", "user.email", "anchor@example.invalid")
            (repo / "src.rs").write_text(
                "// MinecraftServer.java:2-4 and StringReader.java:10-20\n",
                encoding="utf-8",
            )
            _git(repo, "add", "src.rs")
            _git(repo, "commit", "-qm", "initial")
            (repo / "README.md").write_text("unrelated\n", encoding="utf-8")
            _git(repo, "add", "README.md")
            _git(repo, "commit", "-qm", "no anchors in HEAD~1..HEAD")

            env = dict(os.environ)
            env["BBB_VANILLA_ROOT"] = str(vanilla)
            with patch.dict(os.environ, env, clear=True):
                buf = io.StringIO()
                with contextlib.redirect_stdout(buf):
                    code = CHECK.main(["--repo-root", str(repo)])
                output = buf.getvalue()
                self.assertEqual(code, 0, output)
                self.assertIn("2 anchors checked (1 library skipped), 0 problems", output)

                buf_all = io.StringIO()
                with contextlib.redirect_stdout(buf_all):
                    code_all = CHECK.main(["--repo-root", str(repo), "--all"])
                self.assertEqual(code_all, 0, buf_all.getvalue())
                self.assertIn("2 anchors checked (1 library skipped), 0 problems", buf_all.getvalue())


if __name__ == "__main__":
    unittest.main()
